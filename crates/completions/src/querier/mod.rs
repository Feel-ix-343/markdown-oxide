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
    query_metadata: &QueryMetadata,
    cmd_query: &'b BlockLinkCmdQuery,
) -> impl IndexedParallelIterator<Item = LinkBlockCmd<'a>> {
    // let all_cmds = cx.querier().construct_block_link_cmds(cx);
    // let binding = all_cmds.collect::<Vec<_>>();
    // let iterator = binding.into_iter();
    // let matched = run_query(cmd_query, iterator);
    //
    // matched

    vec![].into_par_iter()
}

pub struct Querier<'a> {
    vault: &'a Vault,
}

impl<'a> Querier<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
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
                        ..query_metadata.char_range.end as u32, // TODO is this really cloned?
                };

                let action = |to: EntityReference<'a>| -> UpsertEntityReference<'a> {
                    let type_info = match (
                        &query_metadata.query_syntax_info.syntax_type_info,
                        &to.infile,
                    ) {
                        (QuerySyntaxTypeInfo::Wiki { display }, _) => {
                            ReferenceDisplayMetadataTypeInfo::WikiLink { display: *display }
                        }
                        (
                            QuerySyntaxTypeInfo::Markdown { display: "" },
                            Some(EntityInfileReference::Heading(heading)),
                        ) => ReferenceDisplayMetadataTypeInfo::MDLink { display: heading },
                        (QuerySyntaxTypeInfo::Markdown { display }, _) => {
                            ReferenceDisplayMetadataTypeInfo::MDLink { display }
                        }
                    };
                    UpsertEntityReference {
                        to,
                        in_location: upsert_reference_location,
                        metadata: ReferenceDisplayMetadata {
                            include_md_extension: cx.settings().include_md_extension(),
                            type_info,
                        },
                    }
                };

                let cmd =
                    |label: String, kind: CompletionItemKind, actions| ReferenceNamedSectionCmd {
                        label,
                        kind,
                        cmd_ui_info: cx.entity_viewer().entity_view(it.clone()),
                        actions,
                    };

                let file_name =
                    |path: &Path| path.file_stem().unwrap().to_str().unwrap().to_string();

                match it {
                    Referenceable::File(path, data) => Some(cmd(
                        file_name(path),
                        CompletionItemKind::FILE,
                        action(EntityReference {
                            file: path,
                            infile: None,
                        }),
                    )),
                    Referenceable::Heading(path, data) => Some(cmd(
                        format!("{}#{}", file_name(path), data.heading_text),
                        CompletionItemKind::REFERENCE,
                        action(EntityReference {
                            file: path,
                            infile: Some(EntityInfileReference::Heading(&data.heading_text)),
                        }),
                    )),
                    Referenceable::IndexedBlock(path, data) => Some(cmd(
                        format!("{}#^{}", file_name(path), data.index),
                        CompletionItemKind::REFERENCE,
                        action(EntityReference {
                            file: path,
                            infile: Some(EntityInfileReference::Index(&data.index)),
                        }),
                    )),
                    _ => None,
                }
            })
    }

    // fn construct_block_link_cmds(
    //     &self,
    //     context: &Context,
    // ) -> impl ParallelIterator<Item = LinkBlockCmd> {
    //     let blocks = self.vault.select_blocks();
    //
    //     let filtered = blocks.filter(|it| !it.text.is_empty());
    //
    //     let cmds = filtered
    //         .map(|it| {
    //
    //
    //
    //             let indexed_info = self.indexed_block_info((it.range.end.line, it.range.end.character, it.file));
    //
    //             let index = match indexed_info {
    //                 Some(index) => index,
    //                 None => nanoid!(
    //                     5,
    //                     &['a', 'b', 'c', 'd', 'e', 'f', 'g', '1', '2', '3', '4', '5', '6', '7', '8', '9']
    //                 )
    //
    //             };
    //
    //             let cmd = LinkBlockCmd {
    //                 label: it.text.to_string(),
    //                 kind: match indexed_info {
    //                     Some(_) => CompletionItemKind::REFERENCE,
    //                     None => CompletionItemKind::TEXT
    //                 },
    //                 cmd_ui_info: "".to_string() // TODO,
    //                 actions: (UpsertEntityReference {
    //
    //                     }, AppendBlockIndex {
    //                             index:
    //                         })
    //             };
    //
    //         todo!()
    //         })
    //     todo!();
    //     vec![].into_par_iter()
    // }

    fn indexed_block_info(
        &self,
        location_info: (LineNumber, LastCharacter, &Path),
    ) -> Option<Index> {
        self.vault
            .select_referenceable_nodes(Some(location_info.2))
            .into_par_iter()
            .find_map_any(|it| match it {
                vault::Referenceable::IndexedBlock(_, indexed)
                    if indexed.range.start.line == location_info.0 =>
                {
                    Some(indexed.index.clone())
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
