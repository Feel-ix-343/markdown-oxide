use tracing::{debug, info, instrument, trace};

use std::{collections::HashMap, path::Path, sync::Arc};

use embedder::Embedder;
use index::BlockData;
use md_parser::Documents;
use util::Collection;

use rayon::prelude::*;
pub struct DB {
    index: index::Index,
    embedder: Arc<Embedder>,
}

pub struct DBConfig {
    db_path: &'static Path,
    vault_path: &'static Path,
}

pub struct Score(f32);

impl DB {
    pub async fn new(config: DBConfig) -> anyhow::Result<Self> {
        let embedder = Arc::new(Embedder::new());
        let index = index::Index::init(config, embedder.clone()).await?;
        Ok(Self { index, embedder })
    }

    #[instrument(skip(self))]
    pub async fn query_blocks(
        &self,
        query: &str,
        k: usize,
    ) -> anyhow::Result<Vec<(Arc<BlockData>, Score)>> {
        info!("Querying blocks with query: {}", query);
        let blocks = self.index.all_blocks()?;
        debug!("Retrieved {} blocks from index", blocks.len());

        let embedding = self.embedder.embed(query).await?;
        debug!("Generated embedding for query");

        let mut results = blocks
            .into_par_iter()
            .map(|(block_data, block_embedding)| {
                let similarity = Self::cosine_similarity(&embedding, &block_embedding.0);
                trace!("Calculated similarity: {}", similarity);
                (block_data, Score(similarity))
            })
            .collect::<Vec<_>>();

        let len = results.len();

        results.select_nth_unstable_by(k.min(len) - 1, |a, b| b.1 .0.partial_cmp(&a.1 .0).unwrap());
        results.truncate(k);
        info!("Selected top {} results", results.len());
        Ok(results)
    }

    #[instrument]
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.par_iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.par_iter().map(|x| x * x).sum::<f32>().sqrt();
        let similarity = if magnitude_a == 0.0 || magnitude_b == 0.0 {
            0.0
        } else {
            dot_product / (magnitude_a * magnitude_b)
        };
        trace!("Calculated cosine similarity: {}", similarity);
        similarity
    }
}

mod embedder;
mod index;

mod util {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Collection<T> {
        pub first: T,
        pub rest: Vec<T>,
    }

    impl<T> Collection<T> {
        pub fn iter(&self) -> impl Iterator<Item = &T> {
            std::iter::once(&self.first).chain(self.rest.iter())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio;

    #[tokio::test]
    async fn test_query_blocks() -> anyhow::Result<()> {
        tracing_subscriber::fmt::init();

        let config = DBConfig {
            db_path: Box::leak(Box::new(PathBuf::from("test_db.redb"))),
            vault_path: Box::leak(Box::new(PathBuf::from("/home/felix/notes"))),
        };

        let db = DB::new(config).await?;

        // Test queries
        let queries = vec![
            "College packing list",
            "this is a topic of functional programming",
            "monad",
            "Chinese dynasty",
            "Yuan Dynasty",
            "functional programming semigroupal",
            "the human must understand the code in order to evaluate the LLMs code",
            "ways of socializing in SF",
            "LLM powered coding",
        ];

        for query in queries {
            println!("Query: {}", query);
            let results = db.query_blocks(query, 30).await?;
            for (i, (block, score)) in results.iter().enumerate() {
                println!("Result {}: Score: {:.4}", i + 1, score.0);
                println!("Content: {}", block.content);
                println!();
            }
            println!("----------------------------");
        }

        Ok(())
    }
}
