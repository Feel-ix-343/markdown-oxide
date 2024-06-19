use std::{ops::Range, path::Path};
use vault::{MDHeading, MDIndexedBlock, Referenceable};

pub struct Entity<'a, T: EntityData>(T, Referenceable<'a>, NamedEntityLocationInfo<'a>);

impl<'a> Entity<'a, NamedEntityData<'a>> {
    pub fn from_referenceable(
        referenceable: Referenceable<'a>,
    ) -> Option<Entity<'a, NamedEntityData<'a>>> {
        match referenceable {
            Referenceable::File(path, _) => Some(Entity(
                NamedEntityData {
                    path,
                    type_info: File,
                },
                referenceable,
                NamedEntityLocationInfo {
                    file: path,
                    line_range: None,
                },
            )),
            Referenceable::Heading(
                path,
                MDHeading {
                    heading_text: data,
                    range,
                    ..
                },
            ) => Some(Entity(
                NamedEntityData {
                    path,
                    type_info: Heading(data),
                },
                referenceable,
                NamedEntityLocationInfo {
                    file: path,
                    line_range: Some(range.start.line as usize..range.end.line as usize),
                },
            )),
            Referenceable::IndexedBlock(
                path,
                MDIndexedBlock {
                    index: data, range, ..
                },
            ) => Some(Entity(
                NamedEntityData {
                    path,
                    type_info: IndexedBlock(data),
                },
                referenceable,
                NamedEntityLocationInfo {
                    file: path,
                    line_range: Some(range.start.line as usize..range.end.line as usize),
                },
            )),
            _ => None,
        }
    }
}

impl Entity<'_, NamedEntityData<'_>> {
    pub fn info(&self) -> &NamedEntityData {
        &self.0
    }

    pub fn location_info(&self) -> &NamedEntityLocationInfo {
        &self.2
    }
}

pub struct NamedEntityData<'a> {
    pub path: &'a Path,
    pub type_info: NamedEntityTypeInfo<'a>,
}

use NamedEntityTypeInfo::*;
pub enum NamedEntityTypeInfo<'a> {
    File,
    Heading(&'a str),
    IndexedBlock(&'a str),
}

pub struct NamedEntityLocationInfo<'a> {
    file: &'a Path,
    line_range: Option<Range<usize>>,
}

/// this is a mapping for convient vault api usage. It may be come unnecesasry in the future
impl<'a, T: EntityData> From<&Entity<'a, T>> for Referenceable<'a> {
    fn from(value: &Entity<'a, T>) -> Self {
        value.1.clone() // TODO: ensure that cost is acceptable
    }
}

pub struct UnnamedEntityData;

pub trait EntityData {}
impl EntityData for NamedEntityData<'_> {}
impl EntityData for UnnamedEntityData {}
