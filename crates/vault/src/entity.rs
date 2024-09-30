use anyhow::anyhow;
use itertools::Itertools;
use tracing::{debug, info, instrument};

use crate::embedder::Embeddable;
use crate::md::ContextRange;
use crate::{md, mem_fs};

use std::borrow::Cow;

use std::time::SystemTime;

#[derive(Debug)]
pub enum EntityObject<'a> {
    File(GenericEntityObject<'a, md::File>),
    Heading(GenericEntityObject<'a, md::Heading>),
    Block(GenericEntityObject<'a, md::Block>),
}

impl<'a> EntityObject<'a> {
    pub fn from_file(
        entity: &'a md::File,
        path: &'a str,
        snapshot: mem_fs::Snapshot,
        time: std::time::SystemTime,
    ) -> Self {
        EntityObject::File(GenericEntityObject::from(entity, path, snapshot, time))
    }

    pub fn from_heading(
        entity: &'a md::Heading,
        path: &'a str,
        snapshot: mem_fs::Snapshot,
        time: std::time::SystemTime,
    ) -> Self {
        EntityObject::Heading(GenericEntityObject::from(entity, path, snapshot, time))
    }

    pub fn from_block(
        entity: &'a md::Block,
        path: &'a str,
        snapshot: mem_fs::Snapshot,
        time: std::time::SystemTime,
    ) -> Self {
        EntityObject::Block(GenericEntityObject::from(entity, path, snapshot, time))
    }
}

impl<'a> std::ops::Deref for EntityObject<'a> {
    type Target = dyn EntityObjectInterface + 'a;

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
pub(crate) struct GenericEntityObject<'a, E: Entity> {
    pub(crate) data: &'a E,
    pub(crate) path: &'a str,
    pub(crate) mem_fs_snapshot: mem_fs::Snapshot,
    pub(crate) time: SystemTime,
}

pub trait EntityObjectInterface {
    fn entity_content(&self) -> anyhow::Result<Cow<str>>;
    fn path(&self) -> &str;
}

impl<'a, E: Entity> EntityObjectInterface for GenericEntityObject<'a, E> {
    fn entity_content(&self) -> anyhow::Result<Cow<str>> {
        let range = self.data.location();
        let (_, file) = self.mem_fs_snapshot.get(self.path)?;

        match range {
            EntityLocation::File => Ok(file.text()),
            EntityLocation::Range(range) => file.get_lines(range.start..range.end),
        }
    }
    fn path(&self) -> &str {
        self.path
    }
}

impl<'a> GenericEntityObject<'a, md::Heading> {
    pub fn heading_name(&self) -> anyhow::Result<&str> {
        Ok(&self.data.title)
    }

    pub fn heading_content(&self) -> anyhow::Result<String> {
        self.entity_content().map(|cow| cow.into_owned())
    }
}

impl<'a> GenericEntityObject<'a, md::File> {
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

impl<'a> GenericEntityObject<'a, md::Block> {
    pub fn block_content(&self) -> anyhow::Result<String> {
        self.entity_content().map(|cow| cow.into_owned())
    }
}

impl<'a, E: Entity> GenericEntityObject<'a, E> {
    pub fn from(
        entity: &'a E,
        path: &'a str,
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
        EntityLocation::Range(self.full_range.clone())
    }
}

impl Entity for md::Block {
    fn location(&self) -> EntityLocation {
        EntityLocation::Range(match &self.context_range {
            Some(ContextRange {
                children: Some(range),
                parent: Some(par_range),
            }) => par_range.start..range.end,
            Some(ContextRange {
                children: Some(range),
                parent: None,
            }) => self.range.start..range.end,
            Some(ContextRange {
                children: None,
                parent: Some(range),
            }) => range.start..self.range.end,
            Some(ContextRange {
                children: None,
                parent: None,
            }) => self.range.clone(),
            None => self.range.clone(),
        })
    }
}
