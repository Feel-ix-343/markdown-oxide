use std::{
    collections::HashMap,
    iter,
    path::{Path, PathBuf},
};

pub trait Actions {
    fn actions(&self) -> Vec<&dyn Action>;
}

trait ActionSealed {}

pub trait Action: ActionSealed {
    fn edits(&self) -> Edits;
}

type Edits<'a> = HashMap<&'a Path, Vec<Edit>>;
pub struct Edit {
    pub insert_text: String,
    pub to_area: EditArea,
}

type Line = u32;
type Character = u32;
pub enum EditArea {
    Range { start: Position, end: Position },
    EndOfLine(Line),
}
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Data
#[derive(Clone)]
pub struct UpsertEntityReference<'a> {
    pub to: EntityReference,
    pub in_location: UpsertReferenceLocation<'a>,
    pub metadata: ReferenceDisplayMetadata,
}
#[derive(Clone)]
pub struct EntityReference {
    pub file: PathBuf,
    pub infile: Option<EntityInfileReference>,
}
#[derive(Clone)]
pub struct UpsertReferenceLocation<'a> {
    pub file: &'a Path,
    pub line: u32,
    pub range: std::ops::Range<u32>,
}
#[derive(Clone)]
pub enum EntityInfileReference {
    Heading(String),
    Index(String),
}
#[derive(Clone)]
pub struct ReferenceDisplayMetadata {
    pub include_md_extension: bool,
    pub snippet: bool,
    pub type_info: ReferenceDisplayMetadataTypeInfo,
}
use ReferenceDisplayMetadataTypeInfo::*;
#[derive(Clone)]
pub enum ReferenceDisplayMetadataTypeInfo {
    MDLink { display: String },
    WikiLink { display: Option<String> },
}

impl ActionSealed for UpsertEntityReference<'_> {}
impl Action for UpsertEntityReference<'_> {
    fn edits(&self) -> Edits {
        let file_refname = self
            .to
            .file
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let ref_ = match &self.to.infile {
            None => file_refname,
            Some(EntityInfileReference::Heading(heading)) => format!("{file_refname}#{heading}"),
            Some(EntityInfileReference::Index(index)) => format!("{file_refname}#^{index}"),
        };
        let wrapped_ref = match self.metadata.include_md_extension {
            true => format!("{ref_}.md"),
            false => ref_,
        };

        let link_text: String = match (&self.metadata.type_info, self.metadata.snippet) {
            (MDLink { display }, false) if wrapped_ref.contains(" ") => {
                format!("[{display}](<{wrapped_ref}>)")
            }
            (MDLink { display }, true) if wrapped_ref.contains(" ") => {
                format!("[${{1:{display}}}](<{wrapped_ref}>)${{2:}}")
            }
            (MDLink { display }, false) => {
                format!("[{display}]({wrapped_ref})")
            }
            (MDLink { display }, true) => {
                format!("[${{1:{display}}}]({wrapped_ref})${{2:}}")
            }
            (WikiLink { display: None }, false) => format!("[[{wrapped_ref}]]"),
            (WikiLink { display: None }, true) => format!("[[{wrapped_ref}]]${{1:}}"),
            (
                WikiLink {
                    display: Some(display),
                },
                false,
            ) => format!("[[{wrapped_ref}|{display}]]"),
            (
                WikiLink {
                    display: Some(display),
                },
                true,
            ) => format!("[[{wrapped_ref}|${{1:{display}}}]]${{2:}}"),
        };

        [(
            self.in_location.file,
            vec![Edit {
                insert_text: link_text,
                to_area: EditArea::Range {
                    start: Position {
                        line: self.in_location.line,
                        character: self.in_location.range.start,
                    },
                    end: Position {
                        line: self.in_location.line,
                        character: self.in_location.range.end,
                    },
                },
            }],
        )]
        .into_iter()
        .collect()
    }
}

pub struct AppendBlockIndex<'a> {
    pub index: String,
    pub to_line: u32,
    pub in_file: &'a Path,
}

impl ActionSealed for AppendBlockIndex<'_> {}
impl Action for AppendBlockIndex<'_> {
    fn edits(&self) -> Edits {
        iter::once((
            self.in_file,
            vec![Edit {
                insert_text: format!("     ^{}", self.index),
                to_area: EditArea::EndOfLine(self.to_line),
            }],
        ))
        .collect()
    }
}

impl<A: Action> Actions for A {
    fn actions(&self) -> Vec<&dyn Action> {
        vec![self]
    }
}

impl<A: Action, B: Action> Actions for (A, B) {
    fn actions(&self) -> Vec<&dyn Action> {
        vec![&self.0, &self.1]
    }
}

impl<A: Action, B: Action, C: Action> Actions for (A, B, C) {
    fn actions(&self) -> Vec<&dyn Action> {
        vec![&self.0, &self.1, &self.2]
    }
}

impl<A: Action, B: Action, C: Action, D: Action> Actions for (A, B, C, D) {
    fn actions(&self) -> Vec<&dyn Action> {
        vec![&self.0, &self.1, &self.2, &self.3]
    }
}

impl<A: Action> ActionSealed for Option<A> {}
impl<A: Action> Action for Option<A> {
    fn edits(&self) -> Edits {
        match self {
            None => HashMap::new(),
            Some(a) => a.edits(),
        }
    }
}
