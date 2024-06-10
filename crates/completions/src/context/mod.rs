use std::{ops::Range, path::{Path, PathBuf}};

use crate::completions::Location;
use moxide_config::Settings as MoxideSettings;
use rayon::prelude::*;
use vault::Vault;

pub(crate) struct Context {
}

impl Context {
    pub fn new(vault: &Vault, settings: &MoxideSettings) -> Context {
        todo!()
    }

    pub fn parser(&self) -> &Parser {
        todo!()
    }

    pub fn querier(&self) -> &Querier {
        todo!()
    }

    pub fn entity_view(&self) -> &EntityView {
        todo!()
    }

    pub fn settings(&self) -> &Settings {
        todo!()
    }
}




pub (crate) struct Parser;

impl Parser {
    pub(crate) fn link_completion_info(&self, location: &Location) -> Option<LinkCompletionInfo> {
        todo!()
    }

    pub(crate) fn footnote_completion(&self, location: &Location) -> Option<String> {
        todo!()
    }
    pub(crate) fn link_ref_def_completion(&self, location: &Location) -> Option<String> {
        todo!()
    }
    pub(crate) fn tag(&self, location: &Location) -> Option<Vec<String>> {
        todo!()
    }

    pub(crate) fn entered_query_string(&self) -> &str {
        todo!()
    }

    /// The string that the completion will be replacing. 
    pub(crate) fn entered_completion_string(&self) -> &str {
        todo!()
    }

    pub(crate) fn link_range(&self) -> tower_lsp::lsp_types::Range {
        todo!()
    }

    pub(crate) fn linking_style(&self) -> LinkingStyle {
        todo!()
    }
}

pub(crate) enum LinkingStyle {
    Markdown,
    Wikilink
}


/// Querying the markdown entities in the vault: files, headings, blocks
pub (crate) struct Querier<'fs>(&'fs str);

impl<'fs> Querier<'fs> {
    pub(crate) fn named_grep_query<'a>(&'a self, query: &NamedQuery) -> impl IndexedParallelIterator<Item = NamedEntity> + 'a {
        vec![].into_par_iter()
    }
    // fn named_semantic_query

    // pub(crate) fn unnamed_grep_query(&self, query: &BlockQuery) -> impl IndexedParallelIterator<Item = BlockEntity> {
    //     todo!()
    // }

    // note; this lifetime might be wrong
    pub(crate) fn first_heading_of_file(&self, path: &Path) -> Option<&str> {
        todo!()
    }
}


pub (crate) struct Settings;

impl Settings {
    pub(crate) fn max_query_completion_items(&self) -> usize {
        todo!()
    }

    pub(crate) fn file_as_first_heading(&self) -> bool {
        todo!()
    }

    pub(crate) fn backlinks_to_preview(&self) -> usize {
        todo!()
    }
}

pub(crate) struct EntityView;

impl EntityView {
    // NOTE: the lifetime might be wrong
    pub(crate) fn preview_with_backlinks(&self, entity: &NamedEntity) -> EntityWithBacklinksPreview {
        todo!()
    }
}

pub(crate) struct EntityWithBacklinksPreview<'fs> {
    pub(crate) entity_preview: String,
    pub(crate) backlinks: Box<dyn Iterator<Item = (&'fs Path, String)>>
}

pub(crate) enum PathSettings {
    Full,
    Relative,
    OnlyFileID
}

pub(crate) struct LinkCompletionInfo;

impl LinkCompletionInfo {
    pub(crate) fn line_range(&self) -> Range<usize> {
        todo!()
    }
    pub(crate) fn display_text(&self) -> Option<&str> {
        todo!()
    }
    pub(crate) fn linking_mode(&self) -> LinkingMode {
        todo!()
    }
    pub(crate) fn query(&self) -> &LinkQuery {
        todo!()
    }
}

pub(crate) struct LinkingMode {
    link_type: LinkType,
    ref_type: RefType
}

pub(crate) enum LinkType {
    Markdown,
    Wikilink
}

pub(crate) enum RefType {
    Infile,
    Full
}

pub(crate) struct LinkQuery<'fs> {
    pub(crate) query: Query<'fs>,
}

pub(crate) enum Query<'fs> {
    Named(NamedQuery<'fs>),
    Block(BlockQuery<'fs>)
}


pub (crate) enum Named {
    File,
}

pub (crate) struct NamedQuery<'fs>{
    query_string: NamedQueryString<'fs>,
    // filters: Option<NamedQueryFilters>, // TODO: implement
}


pub(crate) enum NamedQueryString<'fs> {
    Path(PathBuf),
    String(&'fs str),
}

// TODO: implement
pub(crate) struct NamedQueryFilters;



pub (crate) struct BlockQuery<'fs> {
    query_string: &'fs str,
    // filters: Option<UnnamedQueryFilters<'fs>>
}

// pub(crate) struct UnnamedQueryFilters<'fs> {
//     references: Vec<NamedEntitySpecifier<'fs>>,
//     file: Vec<FileSpecifier<'fs>>,
// }

pub(crate) struct NamedEntitySpecifier<'fs> {
    file: FileSpecifier<'fs>,
    infile: Option<InfileSpecifier<'fs>>,
}

pub(crate) enum FileSpecifier<'fs> {
    Path(PathBuf),
    Name(&'fs str),
}

pub(crate) enum InfileSpecifier<'fs> {
    Heading(&'fs str),
    /// Block index without the `^`
    BlockIndex(&'fs str),
}


pub(crate) struct NamedEntity {
}

impl<'fs> NamedEntity {
    pub(crate) fn info(&self) -> NamedEntityInfo<'fs> {
        todo!()
    }
    pub(crate) fn documentation(&self) -> Option<&str> {
        todo!()
    }
}


pub(crate) enum NamedEntityInfo<'a> {
    File {
        path: PathBuf,
        entity_type: FileEntityType<'a>
    },
    Heading {
        heading: &'a str,
        file: PathBuf,
    },
    Block {
        index: &'a str,
        file: PathBuf,
    },
    UnresovledFile {
        file_ref: &'a str,
        entity_type: UnresolvedFileEntityType<'a>
    },
    UnresolvedHeading {
        file_ref: &'a str,
        heading: &'a str
    }
}

impl NamedEntityInfo<'_> {
}

pub(crate) enum FileEntityType<'a> {
    Normal,
    DailyNote {
        relative_label: &'a str
    },
    Alias {
        alias: &'a str
    },
    // Image,
    // anything else?
}


pub(crate) enum UnresolvedFileEntityType<'a> {
    Normal,
    DailyNote {
        relative_label: &'a str
    },
    // anything else?
}

