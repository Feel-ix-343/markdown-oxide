use std::path::Path;

pub trait Actions {
    fn actions(&self) -> Vec<&dyn Action>;
}

trait Action {
    fn edits(&self) -> Vec<TextEdit>;
}

pub struct TextEdit<'a> {
    new_text: String,
    start: (u32, u32),
    end: (u32, u32),
    path: &'a Path,
}

/// Data
pub struct UpsertEntityReference<'a> {
    pub in_file: &'a Path,
    pub on_line: u32,
    pub in_range: std::ops::Range<u32>,
    pub to: EntityInfileReference<'a>,
    pub metadata: ReferenceDisplayMetadata<'a>,
}
pub struct EntityReference<'a> {
    pub file: &'a Path,
    pub infile: Option<EntityInfileReference<'a>>,
    pub with_reference_metadata: ReferenceDisplayMetadata<'a>,
}
pub enum EntityInfileReference<'a> {
    Heading(&'a str),
    Index(&'a str),
}
pub enum ReferenceDisplayMetadata<'a> {
    MDLink {
        display: &'a str,
        include_md_extension: bool,
    },
    WikiLink {
        display: Option<&'a str>,
        include_md_extension: bool,
    },
}
impl Action for UpsertEntityReference<'_> {
    fn edits(&self) -> Vec<TextEdit> {
        todo!()
    }
}

pub struct AppendBlockIndex<'a> {
    pub index: String,
    pub to_line: u32,
    pub in_file: &'a Path,
}

impl Action for AppendBlockIndex<'_> {
    fn edits(&self) -> Vec<TextEdit> {
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
