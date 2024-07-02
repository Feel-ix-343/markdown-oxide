use std::{collections::HashMap, path::Path};

pub trait Actions {
    fn actions(&self) -> Vec<&dyn Action>;
}

trait ActionSealed {}

pub trait Action: ActionSealed {
    fn edits(&self) -> Edits;
}

type Edits<'a> = HashMap<&'a Path, Vec<Edit>>;
pub struct Edit {
    pub new_text: String,
    pub start: Position,
    pub end: Position,
}
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Data
pub struct UpsertEntityReference<'a> {
    pub to: EntityReference<'a>,
    pub in_location: UpsertReferenceLocation<'a>,
    pub metadata: ReferenceDisplayMetadata<'a>,
}
pub struct EntityReference<'a> {
    pub file: &'a Path,
    pub infile: Option<EntityInfileReference<'a>>,
}
pub struct UpsertReferenceLocation<'a> {
    pub file: &'a Path,
    pub line: u32,
    pub range: std::ops::Range<u32>,
}
pub enum EntityInfileReference<'a> {
    Heading(&'a str),
    Index(&'a str),
}
pub struct ReferenceDisplayMetadata<'a> {
    pub include_md_extension: bool,
    pub type_info: ReferenceDisplayMetadataTypeInfo<'a>,
}
use ReferenceDisplayMetadataTypeInfo::*;
pub enum ReferenceDisplayMetadataTypeInfo<'a> {
    MDLink { display: &'a str },
    WikiLink { display: Option<&'a str> },
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
        let ref_ = match self.to.infile {
            None => file_refname,
            Some(EntityInfileReference::Heading(heading)) => format!("{file_refname}#{heading}"),
            Some(EntityInfileReference::Index(index)) => format!("{file_refname}#^{index}"),
        };
        let wrapped_ref = match self.metadata.include_md_extension {
            true => format!("{ref_}.md"),
            false => ref_,
        };

        let link_text: String = match &self.metadata.type_info {
            MDLink { display } if wrapped_ref.contains(" ") => {
                format!("[{display}](<{wrapped_ref}>)")
            }
            MDLink { display } => {
                format!("[{display}]({wrapped_ref})")
            }
            WikiLink { display: None } => format!("[[{wrapped_ref}]]"),
            WikiLink {
                display: Some(display),
            } => format!("[[{wrapped_ref}|{display}]]"),
        };

        [(
            self.in_location.file,
            vec![Edit {
                new_text: link_text,
                start: Position {
                    line: self.in_location.line,
                    character: self.in_location.range.start,
                },
                end: Position {
                    line: self.in_location.line,
                    character: self.in_location.range.end,
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
        todo!()
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
