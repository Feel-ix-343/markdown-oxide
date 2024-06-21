use std::path::Path;
use vault::{MDHeading, MDIndexedBlock, Referenceable};

pub struct Entity<'a> {
    pub info: EntityInfo<'a>,
    referenceable: Referenceable<'a>,
}

pub struct EntityInfo<'a> {
    pub path: &'a Path,
    pub type_info: NamedEntityTypeInfo<'a>,
}

use NamedEntityTypeInfo::*;
pub enum NamedEntityTypeInfo<'a> {
    File,
    Heading(&'a str),
    IndexedBlock(&'a str),
}

impl<'a> Entity<'a> {
    pub fn from_referenceable(referenceable: Referenceable<'a>) -> Option<Entity<'a>> {
        match referenceable {
            Referenceable::File(path, _) => Some(Entity {
                info: EntityInfo {
                    path,
                    type_info: File,
                },
                referenceable,
            }),
            Referenceable::Heading(
                path,
                MDHeading {
                    heading_text: data,
                    range: _,
                    ..
                },
            ) => Some(Entity {
                info: EntityInfo {
                    path,
                    type_info: Heading(data),
                },
                referenceable,
            }),
            Referenceable::IndexedBlock(
                path,
                MDIndexedBlock {
                    index: data,
                    range: _,
                    ..
                },
            ) => Some(Entity {
                info: EntityInfo {
                    path,
                    type_info: IndexedBlock(data),
                },
                referenceable,
            }),
            _ => None,
        }
    }

    pub fn to_referenceable(&self) -> Referenceable<'a> {
        self.referenceable.clone()
    }
}

#[derive(Debug)]
pub struct Block<'a> {
    pub info: UnnamedEntityInfo<'a>,
    line_nr: usize,
    end_char: usize,
    path: &'a Path,
}

#[derive(Debug)]
pub struct UnnamedEntityInfo<'a> {
    pub line_text: &'a str,
}

impl<'a> Block<'a> {
    pub fn from_block(
        line: &'a str,
        line_nr: usize,
        end_char: usize,
        path: &'a Path,
    ) -> Option<Block<'a>> {
        if line.is_empty() {
            return None;
        }

        Some(Block {
            info: UnnamedEntityInfo { line_text: line },
            line_nr,
            path,
            end_char,
        })
    }

    pub fn location_info(&self) -> (LineNumber, LastCharacter, &Path) {
        (self.line_nr, self.end_char, self.path)
    }
}

type LineNumber = usize;
type LastCharacter = usize;
