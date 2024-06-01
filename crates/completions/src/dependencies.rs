use std::{ops::Range, path::PathBuf};

use crate::completions::Location;



pub (crate) struct CompletionQuery<'fs> {
    link_info: LinkInfo<'fs>,
    filter: QueryFilter<'fs>,
    query: Query<'fs>,

    /// Range of the chars in the line
    line_range: Range<usize>
}

pub (crate) enum Query<'fs> {
    Named(NamedQuery<'fs>),
    Unnamed 
}

pub (crate) struct NamedQuery<'fs> {
    link_info: LinkInfo<'fs>,
    filter: QueryFilter<'fs>,
}

pub (crate) struct UnnamedQuery<'fs> {
}

pub (crate) enum CompletionQueryMode {
    Unnamed,
    Named,
    Both
}

pub (crate) struct QueryFilter<'fs> {
    refs: Vec<&'fs str>,
    tagged: Vec<&'fs str>,
}

pub (crate) struct LinkInfo<'fs> {
    display: Option<&'fs str>,
    file_info: FileInfo<'fs>,
    infile_info: Option<&'fs str>,
}

pub (crate) enum FileInfo<'fs> {
    Path(PathBuf),
    Filename(&'fs str)
}

pub (crate) trait Parser {
    fn link_completion_query(&self, location: &Location) -> Option<CompletionQuery>;
    fn footnote(&self, location: &Location) -> Option<String>;
    fn link_ref_def(&self, location: &Location) -> Option<String>;
    fn tag(&self, location: &Location) -> Option<Vec<String>>;
}

pub (crate) enum Named {
    File,
}

pub (crate) trait Querier {
    fn named_query(&self, query: &Query) -> impl IndexedParallelIterator<Item = Box<dyn UsableEntity>>;
    // fn run_query_semantic
}
