use anyhow::anyhow;
use itertools::Itertools;
use tracing::{debug, info, instrument};

use crate::embedder::Embeddable;
use crate::{md, mem_fs};

use std::borrow::Cow;
use std::path::Path;

use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug)]
pub enum EntityObject {
    File(GenericEntityObject<md::File>),
    Heading(GenericEntityObject<md::Heading>),
    Block(GenericEntityObject<md::Block>),
}

pub type Id = (Arc<Path>, EntityLocation);

impl Embeddable<Id> for EntityObject {
    fn content(&self) -> anyhow::Result<Cow<str>> {
        self.entity_content()
    }
    fn id(&self) -> Id {
        self.entity_id()
    }
}

impl EntityObject {
    pub fn from_parsed_file(
        parsed_file: md::ParsedFile,
        path: Arc<Path>,
        time: SystemTime,
        snapshot: mem_fs::Snapshot,
    ) -> Vec<Self> {
        let md::ParsedFile(file, headings, blocks) = parsed_file;

        std::iter::once(Self::from_file(file, path.clone(), snapshot.clone(), time))
            .chain(
                headings.into_iter().map(|heading| {
                    Self::from_heading(heading, path.clone(), snapshot.clone(), time)
                }),
            )
            .chain(
                blocks
                    .into_iter()
                    .map(|block| Self::from_block(block, path.clone(), snapshot.clone(), time)),
            )
            .collect()
    }

    pub fn from_file(
        entity: Arc<md::File>,
        path: Arc<Path>,
        snapshot: mem_fs::Snapshot,
        time: std::time::SystemTime,
    ) -> Self {
        EntityObject::File(GenericEntityObject::from(entity, path, snapshot, time))
    }

    pub fn from_heading(
        entity: Arc<md::Heading>,
        path: Arc<Path>,
        snapshot: mem_fs::Snapshot,
        time: std::time::SystemTime,
    ) -> Self {
        EntityObject::Heading(GenericEntityObject::from(entity, path, snapshot, time))
    }

    pub fn from_block(
        entity: Arc<md::Block>,
        path: Arc<Path>,
        snapshot: mem_fs::Snapshot,
        time: std::time::SystemTime,
    ) -> Self {
        EntityObject::Block(GenericEntityObject::from(entity, path, snapshot, time))
    }
}

impl std::ops::Deref for EntityObject {
    type Target = dyn EntityObjectInterface;

    fn deref(&self) -> &Self::Target {
        match self {
            EntityObject::File(file) => file,
            EntityObject::Heading(heading) => heading,
            EntityObject::Block(block) => block,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum EntityLocation {
    File,
    Range(std::ops::Range<md::Line>),
}
pub(crate) trait Entity {
    fn location(&self) -> EntityLocation;
}

#[derive(Debug)]
pub(crate) struct GenericEntityObject<E: Entity> {
    pub(crate) data: Arc<E>,
    pub(crate) path: Arc<Path>,
    pub(crate) mem_fs_snapshot: mem_fs::Snapshot,
    pub(crate) time: SystemTime,
}

pub trait EntityObjectInterface {
    fn entity_id(&self) -> Id;
    fn entity_content(&self) -> anyhow::Result<Cow<str>>;
    fn path(&self) -> Arc<Path>;
}

impl<E: Entity> EntityObjectInterface for GenericEntityObject<E> {
    fn entity_id(&self) -> Id {
        (self.path.clone(), self.data.location())
    }
    fn entity_content(&self) -> anyhow::Result<Cow<str>> {
        let range = self.data.location();
        let (rope, _) = self.mem_fs_snapshot.get(&self.path)?;

        match range {
            EntityLocation::File => Ok(Cow::Owned(rope.to_string())),
            EntityLocation::Range(range) => {
                let start_byte = rope.line_to_char(range.start);
                let end_byte = rope.line_to_char(range.end);
                Ok(Cow::Owned(
                    rope.get_slice(start_byte..end_byte)
                        .expect("Get slice should not panic")
                        .to_string(),
                ))
            }
        }
    }
    fn path(&self) -> Arc<Path> {
        self.path.clone()
    }
}

impl GenericEntityObject<md::Heading> {
    pub fn heading_name(&self) -> anyhow::Result<String> {
        Ok(self.data.title.clone())
    }

    pub fn heading_content(&self) -> anyhow::Result<String> {
        self.entity_content().map(|cow| cow.into_owned())
    }
}

impl GenericEntityObject<md::File> {
    pub fn file_name(&self) -> anyhow::Result<String> {
        Ok(self
            .path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid file path"))?
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in file name"))?
            .to_string())
    }

    pub fn file_content(&self) -> anyhow::Result<String> {
        self.entity_content().map(|cow| cow.into_owned())
    }
}

impl GenericEntityObject<md::Block> {
    pub fn block_content(&self) -> anyhow::Result<String> {
        self.entity_content().map(|cow| cow.into_owned())
    }
}

impl<E: Entity> GenericEntityObject<E> {
    pub fn from(
        entity: Arc<E>,
        path: Arc<Path>,
        snapshot: mem_fs::Snapshot,
        time: SystemTime,
    ) -> Self {
        Self {
            data: entity,
            path,
            mem_fs_snapshot: snapshot,
            time,
        }
    }

    pub fn into_inner(self) -> anyhow::Result<E> {
        let data = self.data.clone();
        drop(self);
        Arc::into_inner(data).ok_or(anyhow!("Failed to convert arc into inner"))
    }
}

impl Entity for md::File {
    fn location(&self) -> EntityLocation {
        EntityLocation::File
    }
}

impl Entity for md::Heading {
    fn location(&self) -> EntityLocation {
        EntityLocation::Range(self.full_range.clone())
    }
}

impl Entity for md::Block {
    fn location(&self) -> EntityLocation {
        EntityLocation::Range(self.range.clone())
    }
}
