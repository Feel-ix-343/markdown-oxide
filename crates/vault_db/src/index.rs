use itertools::Itertools;
use std::collections::HashSet;

use futures::stream::{self, StreamExt};
use std::collections::HashMap;

use tracing::{info, instrument};

use crate::embedder::Embedder;
use anyhow::Result;
use md_parser::{BorrowedDocBlock, Documents};
use rayon::prelude::*;
use redb::{Database, ReadableTable, TableDefinition, TypeName, Value};
use serde::{Deserialize, Serialize};

use crate::util::Collection;
use crate::DBConfig;

use std::sync::Arc;

use std::path::Path;

pub struct Index {
    db: Database,
}

#[derive(Debug)]
pub struct Row(pub Arc<EntityData>, pub Embedding);

#[derive(Debug)]
pub struct Embedding(pub Vec<f32>);

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockData {
    pub content: String,
    pub children: Vec<BlockData>,
}

impl Index {
    #[instrument(skip(config, embedder))]
    pub async fn init(config: DBConfig, embedder: Arc<Embedder>) -> anyhow::Result<Self> {
        let db_path = config.db_path;

        if !db_path.exists() {
            info!("Creating new database at {:?}", db_path);
            let documents = Documents::from_root_dir(&config.vault_path);
            info!("Loaded {} documents from vault", documents.documents.len());

            let database = Database::create(db_path)?;
            let write_txn = database.begin_write()?;
            {
                let mut table = write_txn.open_table(TABLE)?;

                // Collect all blocks from all documents
                let all_blocks: Vec<_> = documents
                    .documents
                    .into_iter()
                    .flat_map(|(path, document)| {
                        document
                            .all_doc_blocks()
                            .map(|block| (path.clone(), Self::process_block(block)))
                            .collect::<Vec<_>>()
                    })
                    .collect();

                // Process blocks in batches of 2048
                for (batch_index, chunk) in all_blocks.chunks(2048).enumerate() {
                    let unique_docs: HashSet<_> = chunk.iter().map(|(path, _)| path).collect();
                    info!(
                        "Processing batch {} with {} blocks from {} unique documents",
                        batch_index + 1,
                        chunk.len(),
                        unique_docs.len()
                    );

                    let contents: Vec<String> = chunk
                        .iter()
                        .map(|(_, block)| block.content.clone())
                        .collect();

                    let embeddings = embedder.embed_batch(&contents).await?;

                    // Group blocks by document path
                    let grouped_blocks: HashMap<_, Vec<_>> = chunk
                        .iter()
                        .zip(embeddings)
                        .group_by(|((path, _), _)| path)
                        .into_iter()
                        .map(|(path, group)| {
                            (
                                path.clone(),
                                group
                                    .map(|((_, block), embedding)| {
                                        let entity_data = EntityData::Block(block.clone());
                                        (entity_data, embedding)
                                    })
                                    .collect(),
                            )
                        })
                        .collect();

                    // Insert grouped blocks into the database
                    for (path, block_group) in grouped_blocks {
                        table.insert(path.as_os_str().to_str().unwrap(), &block_group)?;
                    }

                    info!(
                        "Completed processing batch {} with {} blocks",
                        batch_index + 1,
                        chunk.len()
                    );
                }
            }
            write_txn.commit()?;

            info!("Database initialized successfully");
            Ok(Self { db: database })
        } else {
            info!("Loading existing database from {:?}", db_path);
            let db = Database::create(db_path)?;
            Ok(Self { db })
        }
    }

    #[instrument(skip(block))]
    fn process_block(block: BorrowedDocBlock) -> BlockData {
        let var_name = match block {
            BorrowedDocBlock::ListBlock(list_block) => {
                let mut content = format!("- {:?}", list_block.content.text.to_string());
                let children = list_block
                    .children
                    .iter()
                    .flat_map(|child_blocks| {
                        child_blocks
                            .iter()
                            .map(|child_block| {
                                Self::process_block(BorrowedDocBlock::ListBlock(child_block))
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                for child in children.iter() {
                    content.push_str("\n");
                    content.push_str(&child.content);
                }

                BlockData { content, children }
            }
            BorrowedDocBlock::ParagraphBlock(paragraph_block) => BlockData {
                content: paragraph_block.content.text.to_string(),
                children: Vec::new(),
            },
        };

        tracing::info!("{var_name:?}");
        var_name
    }

    #[instrument(skip(self))]
    pub fn all_blocks(&self) -> anyhow::Result<Vec<(Arc<BlockData>, Embedding)>> {
        info!("Retrieving all blocks from database");
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TABLE)?;

        let mut result = Vec::new();

        for row in table.iter()? {
            let (_, block_group) = row?;
            for (entity_data, embedding) in block_group.value() {
                if let EntityData::Block(block_data) = entity_data {
                    result.push((Arc::new(block_data), Embedding(embedding)));
                }
            }
        }

        info!("Retrieved {} blocks from database", result.len());
        Ok(result)
    }
}

impl Row {
    pub fn from_db_row(row: RowType) -> Self {
        Self(row.0.into(), Embedding(row.1))
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
