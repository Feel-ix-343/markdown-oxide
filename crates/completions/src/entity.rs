use std::{ops::Range, path::Path};
use vault::{MDHeading, MDIndexedBlock, Referenceable};

pub struct Entity<T: EntityData>(T);

pub trait EntityData {
    type Info;
    fn info(&self) -> &Self::Info;
}

impl<T: EntityData> Entity<T> {
    pub fn info(&self) -> &T::Info {
        self.0.info()
    }
}

pub struct NamedEntityData<'a> {
    info: NamedEntityInfo<'a>,
    referenceable: Referenceable<'a>,
}

pub struct NamedEntityInfo<'a> {
    pub path: &'a Path,
    pub type_info: NamedEntityTypeInfo<'a>,
}

use NamedEntityTypeInfo::*;
pub enum NamedEntityTypeInfo<'a> {
    File,
    Heading(&'a str),
    IndexedBlock(&'a str),
}

impl<'a> EntityData for NamedEntityData<'a> {
    type Info = NamedEntityInfo<'a>;
    fn info(&self) -> &Self::Info {
        &self.info
    }
}

impl<'a> Entity<NamedEntityData<'a>> {
    pub fn from_referenceable(
        referenceable: Referenceable<'a>,
    ) -> Option<Entity<NamedEntityData<'a>>> {
        match referenceable {
            Referenceable::File(path, _) => Some(Entity(NamedEntityData {
                info: NamedEntityInfo {
                    path,
                    type_info: File,
                },
                referenceable,
            })),
            Referenceable::Heading(
                path,
                MDHeading {
                    heading_text: data,
                    range,
                    ..
                },
            ) => Some(Entity(NamedEntityData {
                info: NamedEntityInfo {
                    path,
                    type_info: Heading(data),
                },
                referenceable,
            })),
            Referenceable::IndexedBlock(
                path,
                MDIndexedBlock {
                    index: data, range, ..
                },
            ) => Some(Entity(NamedEntityData {
                info: NamedEntityInfo {
                    path,
                    type_info: IndexedBlock(data),
                },
                referenceable,
            })),
            _ => None,
        }
    }

    pub fn to_referenceable(&self) -> Referenceable<'a> {
        self.0.referenceable.clone()
    }
}

#[derive(Debug)]
pub struct UnnamedEntityData<'a> {
    info: UnnamedEntityInfo<'a>,
    line_nr: usize,
    end_char: usize,
    path: &'a Path,
}

#[derive(Debug)]
pub struct UnnamedEntityInfo<'a> {
    pub line_text: &'a str,
}

impl<'a> EntityData for UnnamedEntityData<'a> {
    type Info = UnnamedEntityInfo<'a>;
    fn info(&self) -> &Self::Info {
        &self.info
    }
}

impl<'a> Entity<UnnamedEntityData<'a>> {
    pub fn from_block(
        line: &'a str,
        line_nr: usize,
        end_char: usize,
        path: &'a Path,
    ) -> Option<Entity<UnnamedEntityData<'a>>> {
        if line.is_empty() {
            return None;
        }

        Some(Entity(UnnamedEntityData {
            info: UnnamedEntityInfo { line_text: line },
            line_nr,
            path,
            end_char,
        }))
    }

    pub fn location_info(&self) -> (LineNumber, LastCharacter, &Path) {
        (self.0.line_nr, self.0.end_char, self.0.path)
    }
}

type LineNumber = usize;
type LastCharacter = usize;
