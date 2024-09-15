use itertools::Itertools;
use redb::{Database, ReadableTable, TableDefinition, TypeName, Value};
use serde::{Deserialize, Serialize};

use crate::util::Collection;
use crate::DBConfig;

use std::sync::Arc;

use std::path::Path;

use std::collections::HashMap;

pub struct Index {
    db: Database,
}

#[derive(Debug)]
pub struct Row(EntityData, Embedding);

#[derive(Debug)]
pub struct Embedding(Vec<f32>);

/// References the row id in an Index Vec
pub type RowIdx = u32;

#[derive(Debug, Serialize, Deserialize)]
pub enum EntityData {
    File(FileData),
    Heading(HeadingData),
    Block(BlockData),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileData {
    pub content: String,
    pub top_level_headings: Vec<RowIdx>,
    pub pre_heading_blocks: Vec<RowIdx>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadingData {
    pub content: String,
    pub parent: Option<RowIdx>,
    pub children: Option<Collection<RowIdx>>,
    pub blocks: Vec<RowIdx>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockData {
    pub content: String,
    pub parent: Option<RowIdx>,
    pub children: Option<Collection<RowIdx>>,
}

impl Index {
    pub fn init(config: DBConfig) -> anyhow::Result<Self> {
        let db_path = config.db_path;

        if !db_path.exists() {
            Ok(Self::parse())
        } else {
            let db = Database::create(db_path)?;
            SElf
        }
    }

    pub fn read_all(&self) -> anyhow::Result<Vec<Row>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TABLE)?;
        let results = table.iter()?;

        results
            .map(|kv| {
                let (_key, value) = kv?;
                Ok(value.value())
            })
            .flatten_ok()
            .map(|result| result.map(Row::from_db_row))
            .collect()
    }

    fn read_from_file(file: &'static Path) -> Self {}

    fn parse(path: &'static Path) -> Index {
    }
}

impl Row {
    pub fn from_db_row(row: RowType) -> Self {
        Self(row.0, Embedding(row.1))
    }
}

type RowType = (EntityData, Vec<f32>);
const TABLE: TableDefinition<&str, Vec<RowType>> = TableDefinition::new("vault-db");

impl Value for EntityData {
    type SelfType<'a> = Self;
    type AsBytes<'a> = Vec<u8>;

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(data).unwrap()
    }

    fn fixed_width() -> Option<usize> {
        None
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        bincode::serialize(value).unwrap()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("EntityData")
    }
}
