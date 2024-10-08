use anyhow::anyhow;
use itertools::Itertools;
use tracing::{debug, info, instrument};

use crate::embedder::Embeddable;
use crate::md::ContextRange;
use crate::{md, mem_fs};

use std::borrow::Cow;

use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug)]
pub enum EntityObject {
    File(GenericEntityObject<md::File>),
    Heading(GenericEntityObject<md::Heading>),
    Block(GenericEntityObject<md::Block>),
}

impl EntityObject {
    pub fn from_file(
        entity: Arc<md::File>,
        path: Arc<str>,
        snapshot: mem_fs::Snapshot,
        time: SystemTime,
    ) -> Self {
        EntityObject::File(GenericEntityObject::from(entity, path, snapshot, time))
    }

    pub fn from_heading(
        entity: Arc<md::Heading>,
        path: Arc<str>,
        snapshot: mem_fs::Snapshot,
        time: SystemTime,
    ) -> Self {
        EntityObject::Heading(GenericEntityObject::from(entity, path, snapshot, time))
    }

    pub fn from_block(
        entity: Arc<md::Block>,
        path: Arc<str>,
        snapshot: mem_fs::Snapshot,
        time: SystemTime,
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
    RangeInclusive(std::ops::RangeInclusive<md::Line>),
}
pub(crate) trait Entity {
    fn location(&self) -> EntityLocation;
}

#[derive(Debug)]
pub(crate) struct GenericEntityObject<E: Entity> {
    pub(crate) data: Arc<E>,
    pub(crate) path: Arc<str>,
    pub(crate) mem_fs_snapshot: mem_fs::Snapshot,
    pub(crate) time: SystemTime,
}

pub trait EntityObjectInterface {
    fn entity_content(&self) -> anyhow::Result<Cow<str>>;
    fn path(&self) -> &str;
}

impl<E: Entity> EntityObjectInterface for GenericEntityObject<E> {
    fn entity_content(&self) -> anyhow::Result<Cow<str>> {
        let range = self.data.location();
        let (_, file) = self.mem_fs_snapshot.get(&self.path)?;

        match range {
            EntityLocation::File => Ok(file.text()),
            EntityLocation::RangeInclusive(range) => file.get_lines(range),
        }
    }
    fn path(&self) -> &str {
        &self.path
    }
}

impl GenericEntityObject<md::Heading> {
    pub fn heading_name(&self) -> anyhow::Result<&str> {
        Ok(&self.data.title)
    }

    pub fn heading_content(&self) -> anyhow::Result<String> {
        self.entity_content().map(|cow| cow.into_owned())
    }
}

impl GenericEntityObject<md::File> {
    pub fn file_name(&self) -> anyhow::Result<&str> {
        self.path
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("Invalid file name"))
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
        path: Arc<str>,
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
}

impl Entity for md::File {
    fn location(&self) -> EntityLocation {
        EntityLocation::File
    }
}

impl Entity for md::Heading {
    fn location(&self) -> EntityLocation {
        EntityLocation::RangeInclusive(self.full_range.start..=self.full_range.end - 1)
    }
}

impl Entity for md::Block {
    fn location(&self) -> EntityLocation {
        EntityLocation::RangeInclusive(match &self.context_range {
            Some(ContextRange {
                children: Some(range),
                parent: Some(par_range),
            }) => *par_range.start()..=*range.end(),
            Some(ContextRange {
                children: Some(range),
                parent: None,
            }) => *self.range.start()..=*range.end(),
            Some(ContextRange {
                children: None,
                parent: Some(range),
            }) => *range.start()..=*self.range.end(),
            Some(ContextRange {
                children: None,
                parent: None,
            }) => self.range.clone(),
            None => self.range.clone(),
        })
    }
}
