use std::{path::Path, sync::Arc};

use derive_deref::Deref;
use embeddings::{Embedder, Vector};
use itertools::Itertools;
use parsing::{BorrowedDocBlock, Document, Documents};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deref)]
pub struct Blocks(Vec<Block>);

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    text: Arc<str>,
}

#[derive(Debug)]
pub enum Topic {
    File(Arc<str>),
    Heading(Arc<str>, Arc<str>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorBlock(pub Block, pub Vector);

#[derive(Debug, Deref, Serialize, Deserialize)]
pub struct VectorBlocks(pub Vec<VectorBlock>);

impl VectorBlock {
    fn new(text: &str, vector: Vector) -> Self {
        let block = Block { text: text.into() };
        Self(block, vector)
    }
}

impl VectorBlocks {
    pub async fn from_blocks(blocks: &Blocks, embedder: Embedder) -> Self {
        let texts = blocks
            .par_iter()
            .map(|it| it.text.as_ref().trim())
            .collect::<Vec<_>>();
        let embeddings = embedder.embeddings(texts).await;
        let vector_blocks = embeddings
            .into_iter()
            .map(|(text, vector)| VectorBlock::new(text, vector))
            .collect();

        Self(vector_blocks)
    }
}

// construct Blocks from Documents
impl Blocks {
    pub fn from_documents(documents: &Documents) -> Blocks {
        let r: Vec<_> = documents
            .documents
            .par_iter()
            .map(|(path, doc)| Self::blocks_from_doc(doc, path.clone()))
            .flatten()
            .collect();
        Self(r)
    }

    fn blocks_from_doc(document: &Document, path: Arc<Path>) -> Vec<Block> {
        document
            .all_doc_blocks()
            .map(|doc_block| Block::from_doc_block(doc_block, path.clone()))
            .collect()
    }
}

impl Block {
    fn from_doc_block(block: BorrowedDocBlock, path: Arc<Path>) -> Self {
        let content = block.content();

        let text = content.text.clone();

        Self { text }
    }
}
