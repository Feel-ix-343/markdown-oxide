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
    settings::DailyNoteDisplay,
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
    let unresolved_cmds =
        cx.querier()
            .construct_unresolved_reference_cmds(cx, query_metadata, cmd_query);
    let daily_notes = cx
        .querier()
        .construct_two_week_daily_note_cmds(cx, query_metadata);
    let binding = all_cmds
        .chain(unresolved_cmds)
        .chain(daily_notes)
        .collect::<Vec<_>>();
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

fn generated_upsert_entity_ref<'a>(
    cx: &Context,
    query_metadata: &QueryMetadata,
    to: EntityReference,
    in_location: UpsertReferenceLocation<'a>,
    generated_display: Option<String>,
) -> UpsertEntityReference<'a> {
    let type_info = match (
        &query_metadata.query_syntax_info.syntax_type_info,
        &to.infile,
        generated_display,
    ) {
        (
            QuerySyntaxTypeInfo::Wiki {
                display: Some(display),
            },
            _,
            _,
        ) => ReferenceDisplayMetadataTypeInfo::WikiLink {
            display: Some(display.to_string()),
        },
        (QuerySyntaxTypeInfo::Wiki { display: None }, _, Some(generated)) => {
            ReferenceDisplayMetadataTypeInfo::WikiLink {
                display: Some(generated),
            }
        }
        (QuerySyntaxTypeInfo::Wiki { display: None }, _, None) => {
            ReferenceDisplayMetadataTypeInfo::WikiLink { display: None }
        }
        (
            QuerySyntaxTypeInfo::Markdown { display: "" },
            Some(EntityInfileReference::Heading(heading)),
            None,
        ) => ReferenceDisplayMetadataTypeInfo::MDLink {
            display: heading.to_string(),
        },
        // Generated takes preference over a heading
        (QuerySyntaxTypeInfo::Markdown { display: "" }, _, Some(generated)) => {
            ReferenceDisplayMetadataTypeInfo::MDLink { display: generated }
        }
        (QuerySyntaxTypeInfo::Markdown { display }, _, _) => {
            ReferenceDisplayMetadataTypeInfo::MDLink {
                display: display.to_string(),
            }
        }
    };

    UpsertEntityReference {
        to,
        metadata: ReferenceDisplayMetadata {
            include_md_extension: cx.settings().include_md_extension(),
            type_info,
        },
        in_location: in_location.clone(),
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
                let upsert_reference_location = query_metadata_ref_location(query_metadata);

                let action =
                    |to: EntityReference, display: Option<String>| -> UpsertEntityReference<'a> {
                        generated_upsert_entity_ref(
                            cx,
                            query_metadata,
                            to,
                            upsert_reference_location.clone(),
                            display,
                        )
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

                let today = chrono::Local::now().date_naive();

                match it {
                    Referenceable::File(path, data) => {
                        let file_entity_ref = EntityReference {
                            file: path.to_path_buf(),
                            infile: None,
                        };
                        Some(
                            [cmd(
                                file_name(path),
                                CompletionItemKind::FILE,
                                action(file_entity_ref.clone(), None),
                                None,
                            )]
                            .into_iter()
                            // add the aliases as commands
                            .chain(data.metadata.iter().map(|it| it.aliases()).flatten().map(
                                |it| {
                                    cmd(
                                        it.to_string(),
                                        CompletionItemKind::ENUM_MEMBER,
                                        action(
                                            file_entity_ref.clone(),
                                            if cx.settings().alias_display_text() {
                                                Some(it.to_string())
                                            } else {
                                                None
                                            },
                                        ),
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
                        action(
                            EntityReference {
                                file: path.to_path_buf(),
                                infile: Some(EntityInfileReference::Heading(
                                    data.heading_text.clone(),
                                )),
                            },
                            None,
                        ),
                        None,
                    )]),
                    Referenceable::IndexedBlock(path, data) => Some(vec![cmd(
                        format!("{}#^{}", file_name(path), data.index),
                        CompletionItemKind::REFERENCE,
                        action(
                            EntityReference {
                                file: path.to_owned(),
                                infile: Some(EntityInfileReference::Index(data.index.clone())),
                            },
                            None,
                        ),
                        None,
                    )]),
                    _ => None,
                }
            })
            .flatten()
    }

    fn construct_two_week_daily_note_cmds(
        &'a self,
        cx: &'a Context<'a>,
        query_metadata: &'a QueryMetadata<'a>,
    ) -> impl ParallelIterator<Item = ReferenceNamedSectionCmd<'a>> {
        let today = chrono::Local::now();
        let path = cx.settings().daily_note_folder_path().to_path_buf();
        (-7..7)
            .into_par_iter()
            .map(move |offset| today + chrono::Duration::days(offset))
            .flat_map(move |day| {
                let file_name = day.format(cx.settings().daily_note_format()).to_string();
                let daily_note_path = path.join(file_name.clone()).with_extension("md");
                let maybe_file = self.vault.md_files.get(&daily_note_path);

                let date_rel_name = match (day - today).num_days() {
                    0 => Some("today".to_string()),
                    1 => Some("tomorrow".to_string()),
                    2..=7 => Some(format!(
                        "next {}",
                        day.format("%A").to_string().to_lowercase()
                    )),
                    -1 => Some("yesterday".to_string()),
                    -7..=-1 => Some(format!(
                        "last {}",
                        day.format("%A").to_string().to_lowercase()
                    )),
                    _ => None,
                }?;

                Some(ReferenceNamedSectionCmd {
                    label: date_rel_name.clone(),
                    kind: CompletionItemKind::EVENT,
                    label_detail: if maybe_file.is_some() {
                        Some(format!("{file_name}.md"))
                    } else {
                        Some("Unresolved".to_string())
                    },
                    cmd_ui_info: if let Some(file) = maybe_file {
                        cx.entity_viewer()
                            .entity_view(&Referenceable::File(&daily_note_path.clone(), file))
                    } else {
                        cx.entity_viewer()
                            .entity_view(&Referenceable::UnresovledFile(
                                daily_note_path.clone(),
                                &file_name,
                            ))
                    },
                    actions: generated_upsert_entity_ref(
                        cx,
                        query_metadata,
                        EntityReference {
                            file: daily_note_path.clone(),
                            infile: None,
                        },
                        query_metadata_ref_location(query_metadata),
                        match cx.settings().daily_note_display_text() {
                            DailyNoteDisplay::WikiAndMD => Some(date_rel_name),
                            DailyNoteDisplay::MD
                                if matches!(
                                    query_metadata.query_syntax_info.syntax_type_info,
                                    QuerySyntaxTypeInfo::Markdown { .. }
                                ) =>
                            {
                                Some(date_rel_name)
                            }
                            DailyNoteDisplay::Wiki
                                if matches!(
                                    query_metadata.query_syntax_info.syntax_type_info,
                                    QuerySyntaxTypeInfo::Wiki { .. }
                                ) =>
                            {
                                Some(date_rel_name)
                            }
                            _ => None,
                        },
                    ),
                })
            })
    }

    fn construct_unresolved_reference_cmds(
        &'a self,
        cx: &'a Context<'a>,
        query_metadata: &'a QueryMetadata<'a>,
        cmd_query: &'a NamedRefCmdQuery<'a>,
    ) -> impl ParallelIterator<Item = ReferenceNamedSectionCmd<'a>> {
        let query_string = cmd_query.to_query_string(); // from the Query implementation in this module
        self.vault
            .select_referenceable_nodes(None)
            .into_par_iter() // TODO: we shuoldn't have to do this
            .flat_map(move |it| match it {
                Referenceable::UnresovledFile(ref path, file_ref) if file_ref != &query_string => {
                    Some(ReferenceNamedSectionCmd {
                        label: file_ref.to_string(),
                        kind: CompletionItemKind::KEYWORD,
                        cmd_ui_info: cx.entity_viewer().entity_view(&it),
                        label_detail: Some("Unresolved File".to_string()),
                        actions: generated_upsert_entity_ref(
                            cx,
                            query_metadata,
                            EntityReference {
                                file: path.to_path_buf(),
                                infile: None,
                            },
                            query_metadata_ref_location(query_metadata),
                            None,
                        ),
                    })
                }
                Referenceable::UnresolvedHeading(ref path, file, heading)
                    if format!("{file}#{heading}") != query_string =>
                {
                    Some(ReferenceNamedSectionCmd {
                        label: format!("{file}#{heading}"),
                        kind: CompletionItemKind::KEYWORD,
                        cmd_ui_info: cx.entity_viewer().entity_view(&it),
                        label_detail: Some("Unresolved Heading".to_string()),
                        actions: generated_upsert_entity_ref(
                            cx,
                            query_metadata,
                            EntityReference {
                                file: path.to_path_buf(),
                                infile: Some(EntityInfileReference::Heading(heading.clone())),
                            },
                            query_metadata_ref_location(query_metadata),
                            None,
                        ),
                    })
                }
                _ => None,
            })
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

            let upsert_entity_reference = generated_upsert_entity_ref(
                cx,
                query_metadata,
                EntityReference {
                    file: it.file.to_path_buf(),
                    infile: Some(EntityInfileReference::Index(index.clone())),
                },
                query_metadata_ref_location(query_metadata),
                if cx.settings().block_compeltions_display_text() {
                    Some(query.grep_string.to_string())
                } else {
                    None
                },
            );
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

fn query_metadata_ref_location<'a>(
    query_metadata: &'a QueryMetadata<'a>,
) -> UpsertReferenceLocation<'a> {
    UpsertReferenceLocation {
        file: query_metadata.path,
        line: query_metadata.line,
        range: query_metadata.char_range.start as u32..query_metadata.char_range.end as u32,
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
