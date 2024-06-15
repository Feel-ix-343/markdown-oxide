use std::{ops::Range, path::Path};
use vault::{MDHeading, MDIndexedBlock, Referenceable};

pub struct NamedEntity<'a>(
    NamedEntityInfo<'a>,
    Referenceable<'a>,
    NamedEntityLocationInfo<'a>,
);

impl<'a> NamedEntity<'a> {
    pub fn from_referenceable(referenceable: Referenceable<'a>) -> Option<NamedEntity<'a>> {
        match referenceable {
            Referenceable::File(path, _) => Some(NamedEntity(
                NamedEntityInfo {
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
            ) => Some(NamedEntity(
                NamedEntityInfo {
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
            ) => Some(NamedEntity(
                NamedEntityInfo {
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

impl NamedEntity<'_> {
    pub fn info(&self) -> &NamedEntityInfo {
        &self.0
    }

    pub fn location_info(&self) -> &NamedEntityLocationInfo {
        &self.2
    }
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

pub struct NamedEntityLocationInfo<'a> {
    file: &'a Path,
    line_range: Option<Range<usize>>,
}

/// this is a mapping for convient vault api usage. It may be come unnecesasry in the future
impl<'a> From<&NamedEntity<'a>> for Referenceable<'a> {
    fn from(value: &NamedEntity<'a>) -> Self {
        value.1.clone() // TODO: ensure that cost is acceptable
    }
}
