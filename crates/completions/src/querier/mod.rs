use std::path::Path;

use crate::{
    command::{
        actions::{
            Actions, AppendBlockIndex, EntityInfileReference, EntityReference,
            ReferenceDisplayMetadata, ReferenceDisplayMetadataTypeInfo, UpsertEntityReference,
            UpsertReferenceLocation,
        },
        Command, LinkBlockCmd, ReferenceNamedSectionCmd,
    },
    context::Context,
    entity::{Block, Entity, NamedEntityTypeInfo::*},
    parser::{
        BlockLinkCmdQuery, EntityInfileQuery, NamedRefCmdQuery, QueryMetadata, QuerySyntaxTypeInfo,
    },
};
use matcher::{run_query, Query, Queryable};
use nanoid::nanoid;
use rayon::prelude::*;
use tower_lsp::lsp_types::CompletionItemKind;
use vault::{Referenceable, Vault};

mod matcher;

pub fn query_named_ref_cmds<'a, 'b: 'a>(
    cx: &'a Context<'a>,
    query_metadata: &'b QueryMetadata,
    cmd_query: &'b NamedRefCmdQuery, // TODO: I think this is a rust bug; lifetime is not needed
) -> impl IndexedParallelIterator<Item = ReferenceNamedSectionCmd<'a>> {
    let all_cmds = cx.querier().construct_named_ref_cmds(cx, query_metadata);
    let binding = all_cmds.collect::<Vec<_>>();
    let iterator = binding.into_iter();
    let matched = run_query(cmd_query, iterator);

    matched.take(cx.settings().num_completions())
}

pub fn query_block_link_cmds<'a, 'b: 'a>(
    cx: &'a Context<'a>,
    query_metadata: &'b QueryMetadata,
    cmd_query: &'b BlockLinkCmdQuery,
) -> impl IndexedParallelIterator<Item = LinkBlockCmd<'a>> {
    let all_cmds = cx
        .querier()
        .construct_block_link_cmds(cx, query_metadata, cmd_query);
    let binding = all_cmds.collect::<Vec<_>>();
    let iterator = binding.into_iter();
    let matched = run_query(cmd_query, iterator);

    matched.take(cx.settings().num_completions())
}

pub struct Querier<'a> {
    vault: &'a Vault,
}

impl<'a> Querier<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}

fn reference_display_metadata_with_type_info(
    cx: &Context,
    type_info: ReferenceDisplayMetadataTypeInfo,
) -> ReferenceDisplayMetadata {
    ReferenceDisplayMetadata {
        include_md_extension: cx.settings().include_md_extension(),
        type_info,
    }
}

impl<'a> Querier<'a> {
    fn construct_named_ref_cmds(
        &self,
        cx: &'a Context, // has lifetime a or greater
        query_metadata: &'a QueryMetadata,
    ) -> impl ParallelIterator<Item = ReferenceNamedSectionCmd<'a>> {
        self.vault
            .select_referenceable_nodes(None)
            .into_par_iter()
            .flat_map(move |it| {
                let upsert_reference_location = UpsertReferenceLocation {
                    file: query_metadata.path,
                    line: query_metadata.line,
                    range: query_metadata.char_range.start as u32
                        ..query_metadata.char_range.end as u32,
                };

                let action_with_default_type_info =
                    |to: EntityReference<'a>| -> UpsertEntityReference<'a> {
                        let type_info = match (
                            &query_metadata.query_syntax_info.syntax_type_info,
                            &to.infile,
                        ) {
                            (
                                QuerySyntaxTypeInfo::Wiki {
                                    display: Some(display),
                                },
                                _,
                            ) => ReferenceDisplayMetadataTypeInfo::WikiLink {
                                display: Some(display.to_string()),
                            },
                            (QuerySyntaxTypeInfo::Wiki { display: None }, _) => {
                                ReferenceDisplayMetadataTypeInfo::WikiLink { display: None }
                            }
                            (
                                QuerySyntaxTypeInfo::Markdown { display: "" },
                                Some(EntityInfileReference::Heading(heading)),
                            ) => ReferenceDisplayMetadataTypeInfo::MDLink {
                                display: heading.to_string(),
                            },
                            (QuerySyntaxTypeInfo::Markdown { display }, _) => {
                                ReferenceDisplayMetadataTypeInfo::MDLink {
                                    display: display.to_string(),
                                }
                            }
                        };
                        UpsertEntityReference {
                            to,
                            in_location: upsert_reference_location.clone(),
                            metadata: reference_display_metadata_with_type_info(cx, type_info),
                        }
                    };

                let cmd =
                    |label: String, kind: CompletionItemKind, actions, detail: Option<String>| {
                        ReferenceNamedSectionCmd {
                            label,
                            kind,
                            label_detail: detail,
                            cmd_ui_info: cx.entity_viewer().entity_view(&it),
                            actions,
                        }
                    };

                let file_name =
                    |path: &Path| path.file_stem().unwrap().to_str().unwrap().to_string();

                match it {
                    Referenceable::File(path, data) => {
                        let file_entity_ref = EntityReference {
                            file: path,
                            infile: None,
                        };
                        Some(
                            [cmd(
                                file_name(path),
                                CompletionItemKind::FILE,
                                action_with_default_type_info(file_entity_ref.clone()),
                                None,
                            )]
                            .into_iter()
                            // add the aliases as commands
                            .chain(data.metadata.iter().map(|it| it.aliases()).flatten().map(
                                |it| {
                                    cmd(
                                        it.to_string(),
                                        CompletionItemKind::ENUM_MEMBER,
                                        UpsertEntityReference {
                                            metadata: reference_display_metadata_with_type_info(
                                                cx,
                                                match query_metadata
                                                    .query_syntax_info
                                                    .syntax_type_info
                                                {
                                                    QuerySyntaxTypeInfo::Wiki { display: None } => {
                                                        ReferenceDisplayMetadataTypeInfo::WikiLink {
                                                            display: Some(it.to_string()),
                                                        }
                                                    }
                                                    QuerySyntaxTypeInfo::Wiki {
                                                        display: Some(display),
                                                    } => {
                                                        ReferenceDisplayMetadataTypeInfo::WikiLink {
                                                            display: Some(display.to_string()),
                                                        }
                                                    }
                                                    QuerySyntaxTypeInfo::Markdown {
                                                        display: "",
                                                    } => ReferenceDisplayMetadataTypeInfo::MDLink {
                                                        display: it.to_string(),
                                                    },
                                                    QuerySyntaxTypeInfo::Markdown { display } => {
                                                        ReferenceDisplayMetadataTypeInfo::MDLink {
                                                            display: display.to_string(),
                                                        }
                                                    }
                                                },
                                            ),
                                            to: file_entity_ref.clone(),
                                            in_location: upsert_reference_location.clone(),
                                        },
                                        Some(format!("Alias for {}.md", file_name(path))),
                                    )
                                },
                            ))
                            .collect::<Vec<_>>(),
                        )
                    }
                    Referenceable::Heading(path, data) => Some(vec![cmd(
                        format!("{}#{}", file_name(path), data.heading_text),
                        CompletionItemKind::REFERENCE,
                        action_with_default_type_info(EntityReference {
                            file: path,
                            infile: Some(EntityInfileReference::Heading(data.heading_text.clone())),
                        }),
                        None,
                    )]),
                    Referenceable::IndexedBlock(path, data) => Some(vec![cmd(
                        format!("{}#^{}", file_name(path), data.index),
                        CompletionItemKind::REFERENCE,
                        action_with_default_type_info(EntityReference {
                            file: path,
                            infile: Some(EntityInfileReference::Index(data.index.clone())),
                        }),
                        None,
                    )]),
                    _ => None,
                }
            })
            .flatten()
    }

    fn construct_block_link_cmds(
        &'a self,
        cx: &'a Context,
        query_metadata: &'a QueryMetadata,
        query: &'a BlockLinkCmdQuery,
    ) -> impl ParallelIterator<Item = LinkBlockCmd<'a>> {
        let blocks = self.vault.select_blocks();

        let filtered = blocks
            .filter(|it| !it.text.is_empty())
            .filter(|it| it.range.end.line != query_metadata.line);

        let cmds = filtered.map(|it| {
            let indexed_info =
                self.indexed_block_info((it.range.end.line, it.range.end.character, it.file));

            let index = match &indexed_info {
                Some((index, _)) => index.to_string(),
                None => nanoid!(
                    5,
                    &[
                        'a', 'b', 'c', 'd', 'e', 'f', 'g', '1', '2', '3', '4', '5', '6', '7', '8',
                        '9'
                    ]
                ),
            };

            let upsert_entity_reference = UpsertEntityReference {
                to: EntityReference {
                    file: it.file,
                    infile: Some(EntityInfileReference::Index(index.clone())),
                },
                metadata: ReferenceDisplayMetadata {
                    type_info: match (
                        &query_metadata.query_syntax_info.syntax_type_info,
                        query.grep_string,
                    ) {
                        (QuerySyntaxTypeInfo::Wiki { display: None }, "") => {
                            ReferenceDisplayMetadataTypeInfo::WikiLink { display: None }
                        }
                        (QuerySyntaxTypeInfo::Wiki { display: None }, grep_string) => {
                            ReferenceDisplayMetadataTypeInfo::WikiLink {
                                display: Some(grep_string.to_owned()),
                            }
                        }
                        (
                            QuerySyntaxTypeInfo::Wiki {
                                display: Some(display),
                            },
                            _,
                        ) => ReferenceDisplayMetadataTypeInfo::WikiLink {
                            display: Some(display.to_string()),
                        },
                        (QuerySyntaxTypeInfo::Markdown { display: "" }, "") => {
                            ReferenceDisplayMetadataTypeInfo::MDLink {
                                display: "".to_owned(),
                            }
                        }
                        (QuerySyntaxTypeInfo::Markdown { display: "" }, grep_string) => {
                            ReferenceDisplayMetadataTypeInfo::MDLink {
                                display: grep_string.to_owned(),
                            }
                        }
                        (QuerySyntaxTypeInfo::Markdown { display }, _) => {
                            ReferenceDisplayMetadataTypeInfo::MDLink {
                                display: display.to_string(),
                            }
                        }
                    },
                    include_md_extension: cx.settings().include_md_extension(),
                },
                in_location: UpsertReferenceLocation {
                    file: query_metadata.path,
                    line: query_metadata.line,
                    range: query_metadata.char_range.start as u32
                        ..query_metadata.char_range.end as u32,
                },
            };
            let cmd = LinkBlockCmd {
                label: it.text.to_string(),
                kind: match &indexed_info {
                    Some(_) => CompletionItemKind::REFERENCE,
                    None => CompletionItemKind::TEXT,
                },
                label_detail: Some(it.file.file_name().unwrap().to_str().unwrap().to_string()),
                cmd_ui_info: match indexed_info {
                    Some((_, ref referenceable)) => cx.entity_viewer().entity_view(referenceable),
                    None => cx.entity_viewer().unindexed_block_entity_view(&it),
                },
                actions: (
                    upsert_entity_reference,
                    match indexed_info {
                        None => Some(AppendBlockIndex {
                            index: index.to_string(),
                            in_file: it.file,
                            to_line: it.range.start.line,
                        }),
                        Some(_) => None,
                    },
                ),
            };

            cmd
        });

        cmds
    }

    fn indexed_block_info(
        &'a self,
        location_info: (LineNumber, LastCharacter, &'a Path),
    ) -> Option<(Index, Referenceable<'a>)> {
        self.vault
            .select_referenceable_nodes(Some(location_info.2))
            .into_par_iter()
            .find_map_any(|it| match it {
                vault::Referenceable::IndexedBlock(_, indexed)
                    if indexed.range.start.line == location_info.0 =>
                {
                    Some((indexed.index.clone(), it.clone()))
                }
                _ => None,
            })
    }
}

type LineNumber = u32;
type LastCharacter = u32;
type Index = String;

impl Query for BlockLinkCmdQuery<'_> {
    fn to_query_string(&self) -> String {
        self.grep_string.to_string()
    }
}

impl Query for NamedRefCmdQuery<'_> {
    fn to_query_string(&self) -> String {
        match self {
            NamedRefCmdQuery {
                file_query: file_ref,
                infile_query: None,
            } => file_ref.to_string(),
            NamedRefCmdQuery {
                file_query: file_ref,
                infile_query: Some(EntityInfileQuery::Heading(heading_string)),
            } => format!("{file_ref}#{heading_string}"),
            NamedRefCmdQuery {
                file_query: file_ref,
                infile_query: Some(EntityInfileQuery::Index(index)),
            } => format!("{file_ref}#^{index}"),
        }
    }
}

impl Queryable for Entity<'_> {
    fn match_string(&self) -> String {
        let file_ref = self.info.path.file_name().unwrap().to_str().unwrap();

        match self.info.type_info {
            Heading(heading) => format!("{file_ref}#{heading}"),
            IndexedBlock(index) => format!("{file_ref}#^{index}"),
            _ => file_ref.to_string(),
        }
    }
}

impl Queryable for Block<'_> {
    fn match_string(&self) -> String {
        self.info.line_text.to_string()
    }
}

impl<A: Actions> Queryable for Command<A> {
    fn match_string(&self) -> String {
        self.label.to_string()
    }
}
