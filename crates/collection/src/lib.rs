use embeddings::{Embedder, Vector};
use itertools::Itertools;
use model::{Block, VectorBlock, VectorBlocks};
use rayon::prelude::*;
use std::sync::Arc;

pub struct Query(Arc<str>);

pub struct VectorQuery(Query, Vector);

pub struct VectorSearch;
#[derive(PartialEq, PartialOrd, Debug)]
pub struct Rank(f32);
#[derive(Debug)]
pub struct SearchResult<'a>(Vec<(Rank, &'a Block)>);

impl VectorQuery {
    pub async fn new(query: Query, embedder: Embedder) -> Self {
        let vector = embedder.embedding(query.0.as_ref()).await.1;

        Self(query, vector)
    }
}

impl VectorSearch {
    pub fn query<'a>(
        &self,
        query: &VectorQuery,
        vector_blocks: &'a VectorBlocks,
        items: usize,
    ) -> SearchResult<'a> {
        let mut list = vector_blocks
            .par_iter()
            .map(|v| {
                let sim = Self::cosine_similarity(&query.1, &v.1);

                (Rank(sim), &v.0)
            })
            .collect::<Vec<_>>();

        list.par_sort_by(|(rank1, _), (rank2, _)| rank2.partial_cmp(rank1).unwrap());
        list.truncate(items);

        SearchResult(list)
    }

    fn cosine_similarity(v1: &Vector, v2: &Vector) -> f32 {
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let magnitude_v1: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_v2: f32 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot_product / (magnitude_v1 * magnitude_v2)
    }
}

#[cfg(test)]
mod model_tests {
    use std::{collections::HashMap, path::PathBuf, str::FromStr};

    use embeddings::Embedder;
    use model::{Blocks, VectorBlocks};
    use parsing::Documents;
    use rayon::prelude::*;

    use crate::{Query, VectorQuery, VectorSearch};

    #[test]
    fn bench() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let now = std::time::Instant::now();
            // let path = PathBuf::from_str("/home/felix/coding/LargerIdeas/MarkdownOxide/markdown-oxide/TestFiles").unwrap();
            let path = PathBuf::from_str("/home/felix/notes").unwrap();
            let documents = Documents::from_root_dir(&path);

            println!("Documents: {:?}", now.elapsed());

            let blocks = Blocks::from_documents(&documents);

            println!("Blocks: {:?}", now.elapsed());

            let embedder = Embedder::new().await;

            // if not in a file, calculate and write to the file vector blocks
            let vector_blocks = if let Ok(file) = std::fs::File::open("vector_blocks.bin") {
                println!("Reading from file");
                let r = bincode::deserialize_from(file).unwrap();
                println!("Read from file: {:?}", now.elapsed());
                r
            } else {
                println!("Calculating");
                let vb = VectorBlocks::from_blocks(&blocks, embedder).await;
                println!("Embeddings: {:?}", now.elapsed());
                let file = std::fs::File::create("vector_blocks.bin").unwrap();
                bincode::serialize_into(file, &vb).unwrap();
                println!("Wrote to file {:?}", now.elapsed());

                vb
            };

            let searcher = VectorSearch;

            let queries = vec!["Packing list for college", "Urgent tasks", "#issue"];
            for query in queries {
                let vector_query = VectorQuery::new(Query(query.into()), embedder).await;
                let r = searcher.query(&vector_query, &vector_blocks, 15);
                println!("Search for '{}': {:?}", query, now.elapsed());
                println!("Results for '{}': {:#?}", query, r);
            }

            let elapsed = now.elapsed();
            println!("Total Elapsed: {:?}", elapsed);
            assert!(elapsed.as_secs() < 1);
        });

        Ok(())
    }
}
