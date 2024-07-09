use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    command::{
        actions::{
            Actions, AppendBlockIndex, EntityInfileReference, EntityReference,
            ReferenceDisplayMetadata, ReferenceDisplayMetadataTypeInfo, UpsertEntityReference,
            UpsertReferenceLocation,
        },
        Command, LinkBlockCmd, ReferenceNamedSectionCmd,
    },
    context::QueryContext,
    entity::{Block, Entity, NamedEntityTypeInfo::*},
    parser::{
        BlockLinkCmdQuery, EntityInfileQuery, NamedRefCmdQuery, QueryMetadata, QuerySyntaxTypeInfo,
    },
    settings::DailyNoteDisplay,
};
use matcher::{run_query, run_query_on_par_iter, Query, Queryable};
use nanoid::nanoid;
use rayon::{iter, prelude::*};
use tower_lsp::lsp_types::CompletionItemKind;
use vault::{Block as VaultBlock, MDFile, MDHeading, MDIndexedBlock, Referenceable, Vault};

mod matcher;

pub fn query_named_ref_cmds(
    cx: &QueryContext,
    query_metadata: &QueryMetadata,
    cmd_query: &NamedRefCmdQuery, // TODO: I think this is a rust bug; lifetime is not needed
    data: &Vec<NamedSection>,
) -> Vec<ReferenceNamedSectionCmd> {
    let binding = cx.querier();
    let all_cmds =
        binding.construct_fundamental_section_ref_cmds(cx, query_metadata, cmd_query, data);
    let binding = cx.querier();
    let daily_notes = binding.construct_two_week_daily_note_cmds(cx, query_metadata);
    let binding = iter::empty()
        .chain(daily_notes)
        .chain(all_cmds)
        .collect::<Vec<_>>();
    let iterator = binding.into_iter();
    let matched = run_query(cmd_query, iterator);

    matched.take(cx.settings().num_completions()).collect()
}

pub fn query_block_link_cmds<'a>(
    cx: &'a QueryContext<'a, 'a>,
    query_metadata: &'a QueryMetadata,
    cmd_query: &'a BlockLinkCmdQuery,
    data: &Vec<VaultBlock>,
) -> Vec<LinkBlockCmd> {
    let binding = cx.querier();
    let all_cmds = binding.construct_block_link_cmds(cx, query_metadata, cmd_query, data);
    // let binding = all_cmds.collect::<Vec<_>>();
    // let iterator = binding.into_iter();
    // let matched = run_query(cmd_query, iterator);
    //
    // matched.take(cx.settings().num_completions())
    all_cmds.collect()
}

#[derive(Clone, Copy)]
pub struct Querier<'a> {
    vault: &'a Vault,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct QuerierCache {
    pub blocks: Option<Arc<Vec<VaultBlock>>>,
    pub named_sections: Option<Arc<Vec<NamedSection>>>,
}
impl QuerierCache {
    // pub(crate) fn named_sections(&mut self, cx: &QueryContext) -> Arc<Vec<NamedSection>> {
    //     match &self.named_sections {
    //         None => {
    //             let named_sections = Arc::new(cx.querier().get_named_sections());
    //             self.named_sections = Some(named_sections.clone());
    //             named_sections
    //         }
    //         Some(named_sections) => named_sections.clone(),
    //     }
    // }
    // pub(crate) fn blocks(&mut self, cx: &QueryContext) -> Arc<Vec<VaultBlock>> {
    //     match &self.blocks {
    //         None => {
    //             let blocks = Arc::new(cx.querier().get_blocks());
    //             self.blocks = Some(blocks.clone());
    //             blocks
    //         }
    //         Some(blocks) => blocks.clone(),
    //     }
    // }

    pub(crate) fn clear(&mut self) {
        self.blocks = None;
        self.named_sections = None;
    }
}

/// Also cachable
#[derive(Debug, Clone)]
pub enum NamedSection {
    File(PathBuf, Arc<MDFile>),
    Heading(PathBuf, Arc<MDHeading>),
    IndexedBlock(PathBuf, Arc<MDIndexedBlock>),
    UnresovledFile(PathBuf, String),
    UnresolvedHeading(PathBuf, String, String),
    UnresovledIndexedBlock(PathBuf, String, String),
}
impl NamedSection {
    fn to_referenceable(&self) -> Referenceable {
        match &self {
            Self::File(path, mdfile) => Referenceable::File(path, mdfile),
            Self::Heading(path, heading) => Referenceable::Heading(path, heading),
            Self::IndexedBlock(path, data) => Referenceable::IndexedBlock(path, data),
            Self::UnresovledFile(path, string) => {
                Referenceable::UnresovledFile(path.to_path_buf(), string)
            }
            Self::UnresolvedHeading(path, file, heading) => {
                Referenceable::UnresolvedHeading(path.to_path_buf(), file, heading)
            }
            Self::UnresovledIndexedBlock(path, file, index) => {
                Referenceable::UnresovledIndexedBlock(path.to_path_buf(), file, index)
            }
        }
    }
}

impl<'a> Querier<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}

fn generated_upsert_entity_ref(
    cx: &QueryContext,
    query_metadata: &QueryMetadata,
    to: EntityReference,
    in_location: UpsertReferenceLocation,
    generated_display: Option<String>,
) -> UpsertEntityReference {
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
            QuerySyntaxTypeInfo::Markdown { display },
            Some(EntityInfileReference::Heading(heading)),
            None,
        ) if display == "" => ReferenceDisplayMetadataTypeInfo::MDLink {
            display: heading.to_string(),
        },
        // Generated takes preference over a heading
        (QuerySyntaxTypeInfo::Markdown { display }, _, Some(generated)) if display == "" => {
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
    pub fn get_named_sections(&self) -> Vec<NamedSection> {
        self.vault
            .select_referenceable_nodes(None)
            .into_iter()
            .filter(|it| {
                matches!(
                    it,
                    Referenceable::File(..)
                        | Referenceable::Heading(..)
                        | Referenceable::IndexedBlock(..)
                        | Referenceable::UnresovledFile(..)
                        | Referenceable::UnresolvedHeading(..)
                        | Referenceable::UnresovledIndexedBlock(..)
                )
            })
            .flat_map(|it| match it {
                Referenceable::File(path, file) => Some(NamedSection::File(
                    path.as_path().into(),
                    Arc::new(file.clone()),
                )),
                Referenceable::Heading(path, heading) => Some(NamedSection::Heading(
                    path.as_path().into(),
                    Arc::new(heading.clone()),
                )),
                Referenceable::IndexedBlock(path, data) => Some(NamedSection::IndexedBlock(
                    path.as_path().into(),
                    Arc::new(data.clone()),
                )),
                Referenceable::UnresovledFile(path, string) => Some(NamedSection::UnresovledFile(
                    path.as_path().into(),
                    string.to_string(),
                )),
                Referenceable::UnresolvedHeading(path, file, heading) => {
                    Some(NamedSection::UnresolvedHeading(
                        path.as_path().into(),
                        file.to_string(),
                        heading.to_string(),
                    ))
                }
                Referenceable::UnresovledIndexedBlock(path, file, index) => {
                    Some(NamedSection::UnresovledIndexedBlock(
                        path.as_path().into(),
                        file.to_string(),
                        index.to_string(),
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    fn construct_fundamental_section_ref_cmds(
        &self,
        cx: &'a QueryContext, // has lifetime a or greater
        query_metadata: &'a QueryMetadata,
        query: &'a NamedRefCmdQuery,
        data: &'a Vec<NamedSection>,
    ) -> impl ParallelIterator<Item = ReferenceNamedSectionCmd> + '_ {
        let query_string = query.to_query_string(); // from the Query implementation in this module
        let queried = run_query(query, data.iter());

        queried
            .take(cx.settings().num_completions())
            .flat_map(move |it| {
                let upsert_reference_location = query_metadata_ref_location(query_metadata);

                let action =
                    |to: EntityReference, display: Option<String>| -> UpsertEntityReference {
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
                            cmd_ui_info: cx.entity_viewer().entity_view(it.to_referenceable()),
                            actions,
                        }
                    };

                let file_name =
                    |path: &Path| path.file_stem().unwrap().to_str().unwrap().to_string();

                match &it {
                    NamedSection::File(path, _data) => {
                        let file_entity_ref = EntityReference {
                            file: path.to_path_buf().into(),
                            infile: None,
                        };
                        Some(cmd(
                            file_name(&path),
                            CompletionItemKind::FILE,
                            action(file_entity_ref.clone(), None),
                            None,
                        ))
                    }
                    NamedSection::Heading(path, data) => Some(cmd(
                        format!("{}#{}", file_name(&path), data.heading_text),
                        CompletionItemKind::REFERENCE,
                        action(
                            EntityReference {
                                file: path.to_path_buf().into(),
                                infile: Some(EntityInfileReference::Heading(
                                    data.heading_text.clone(),
                                )),
                            },
                            None,
                        ),
                        None,
                    )),
                    NamedSection::IndexedBlock(path, data) => Some(cmd(
                        format!("{}#^{}", file_name(&path), data.index),
                        CompletionItemKind::REFERENCE,
                        action(
                            EntityReference {
                                file: path.to_path_buf().into(),
                                infile: Some(EntityInfileReference::Index(data.index.clone())),
                            },
                            None,
                        ),
                        None,
                    )),
                    NamedSection::UnresovledFile(ref path, file_ref)
                        if *file_ref != query_string =>
                    {
                        Some(ReferenceNamedSectionCmd {
                            label: file_ref.to_string(),
                            kind: CompletionItemKind::KEYWORD,
                            cmd_ui_info: cx.entity_viewer().entity_view(it.to_referenceable()),
                            label_detail: Some("Unresolved File".to_string()),
                            actions: generated_upsert_entity_ref(
                                cx,
                                query_metadata,
                                EntityReference {
                                    file: path.clone().into(),
                                    infile: None,
                                },
                                query_metadata_ref_location(query_metadata),
                                None,
                            ),
                        })
                    }
                    NamedSection::UnresolvedHeading(ref path, file, heading)
                        if format!("{file}#{heading}") != query_string =>
                    {
                        Some(ReferenceNamedSectionCmd {
                            label: format!("{file}#{heading}"),
                            kind: CompletionItemKind::KEYWORD,
                            cmd_ui_info: cx.entity_viewer().entity_view(it.to_referenceable()),
                            label_detail: Some("Unresolved Heading".to_string()),
                            actions: generated_upsert_entity_ref(
                                cx,
                                query_metadata,
                                EntityReference {
                                    file: path.clone().into(),
                                    infile: Some(EntityInfileReference::Heading(heading.clone())),
                                },
                                query_metadata_ref_location(query_metadata),
                                None,
                            ),
                        })
                    }
                    _ => None,
                }
            })
    }

    fn construct_two_week_daily_note_cmds(
        self,
        cx: &'a QueryContext,
        query_metadata: &'a QueryMetadata,
    ) -> impl ParallelIterator<Item = ReferenceNamedSectionCmd> + 'a {
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
                            .entity_view(Referenceable::File(&daily_note_path.clone(), file))
                    } else {
                        cx.entity_viewer()
                            .entity_view(Referenceable::UnresovledFile(
                                daily_note_path.clone(),
                                &file_name,
                            ))
                    },
                    actions: generated_upsert_entity_ref(
                        cx,
                        query_metadata,
                        EntityReference {
                            file: daily_note_path.clone().into(),
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

    pub fn get_blocks(&self, query_metadata: &QueryMetadata) -> Vec<VaultBlock> {
        self.vault
            .select_blocks()
            .filter(|it| !it.text.trim().is_empty())
            .filter(|it| it.range.end.line != query_metadata.line)
            .collect()
    }

    fn construct_block_link_cmds(
        self,
        cx: &'a QueryContext,
        query_metadata: &'a QueryMetadata,
        query: &'a BlockLinkCmdQuery,
        data: &'a Vec<VaultBlock>,
    ) -> impl IndexedParallelIterator<Item = LinkBlockCmd> + 'a {
        let blocks = data;

        let matched = run_query(query, blocks.into_iter());

        let cmds = matched
            .take(cx.settings().num_block_completions())
            .map(move |it| {
                let indexed_info =
                    self.indexed_block_info((it.range.end.line, it.range.end.character, &it.file));

                let index = match &indexed_info {
                    Some((index, _)) => index.to_string(),
                    None => nanoid!(
                        5,
                        &[
                            'a', 'b', 'c', 'd', 'e', 'f', 'g', '1', '2', '3', '4', '5', '6', '7',
                            '8', '9'
                        ]
                    ),
                };

                let upsert_entity_reference = generated_upsert_entity_ref(
                    cx,
                    query_metadata,
                    EntityReference {
                        file: it.file.clone(),
                        infile: Some(EntityInfileReference::Index(index.clone())),
                    },
                    query_metadata_ref_location(query_metadata),
                    if cx.settings().block_compeltions_display_text() {
                        Some(query.display_grep_string().to_string())
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
                        Some((_, ref referenceable)) => {
                            cx.entity_viewer().entity_view(referenceable.clone())
                        }
                        None => cx.entity_viewer().unindexed_block_entity_view(&it),
                    },
                    actions: (
                        upsert_entity_reference,
                        match indexed_info {
                            None => Some(AppendBlockIndex {
                                index: index.to_string(),
                                in_file: it.file.clone(),
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

fn query_metadata_ref_location<'a>(query_metadata: &'a QueryMetadata) -> UpsertReferenceLocation {
    UpsertReferenceLocation {
        file: query_metadata.path.as_path().into(),
        line: query_metadata.line,
        range: query_metadata.char_range.start as u32..query_metadata.char_range.end as u32,
    }
}

type LineNumber = u32;
type LastCharacter = u32;
type Index = String;

impl Query for BlockLinkCmdQuery {
    fn to_query_string(&self) -> String {
        self.grep_string().to_string()
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

impl Queryable for &NamedSection {
    fn match_string(&self) -> String {
        let file_name = |path: &Path| path.file_stem().unwrap().to_str().unwrap().to_string();
        match self {
            NamedSection::File(path, _) => file_name(path),
            NamedSection::Heading(path, heading) => {
                format!("{}#{}", file_name(path), heading.heading_text)
            }
            NamedSection::IndexedBlock(path, index_data) => {
                format!("{}#^{}", file_name(path), index_data.index)
            }
            NamedSection::UnresovledFile(_path, string) => string.to_string(),
            NamedSection::UnresolvedHeading(_path, file_ref, heading) => {
                format!("{file_ref}#{heading}")
            }
            NamedSection::UnresovledIndexedBlock(_path, file_ref, index) => {
                format!("{}#^{}", file_ref, index)
            }
        }
    }
}

impl Queryable for &vault::Block {
    fn match_string(&self) -> String {
        self.text.to_string()
    }
}
