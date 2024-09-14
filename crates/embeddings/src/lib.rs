use async_openai::config::OpenAIConfig;
use async_openai::types::CreateEmbeddingRequest;
use async_openai::Client;
use derive_deref::Deref;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deref, Serialize, Deserialize)]
pub struct Vector(Vec<f32>);

#[derive(Clone, Copy)]
pub struct Embedder {
    client: &'static Client<OpenAIConfig>,
}

const DIMENSIONS: u32 = 3072;
impl Embedder {
    pub async fn embeddings<'a>(&self, texts: Vec<&'a str>) -> Vec<(&'a str, Vector)> {
        println!("Number of texts: {}", texts.len());

        let tokenizer = tiktoken_rs::cl100k_base().unwrap();
        let texts = texts
            .into_iter()
            .flat_map(|it| {
                let tokens = tokenizer.split_by_token_iter(it, false).try_len().ok()?;
                if tokens <= 8191 && !it.is_empty() {
                    Some(it)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        println!("Number of texts after filtering: {}", texts.len());

        // Split texts into chunks of 2048
        let chunks: Vec<Vec<&'a str>> = texts.chunks(2048).map(|chunk| chunk.to_vec()).collect();

        // print num number of chunks
        println!("Number of chunks: {}", chunks.len());

        let mut results = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            println!("Processing chunk {} of size: {}", i + 1, chunk.len());
            let request = CreateEmbeddingRequest {
                model: "text-embedding-3-large".to_string(),
                input: chunk.clone().into(),
                dimensions: Some(DIMENSIONS),
                ..Default::default()
            };

            let response = self.client.embeddings().create(request).await.unwrap();

            let chunk_results: Vec<(&'a str, Vector)> = response
                .data
                .into_iter()
                .map(|e| (chunk[e.index as usize], Vector(e.embedding)))
                .collect();

            results.extend(chunk_results);

            // if (i + 1) % 8 == 0 {
            //     let seconds = 2;
            //     println!("Waiting for {seconds} seconds...");
            //     tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
            // }
        }

        results
    }

    pub async fn embedding<'a>(&self, text: &'a str) -> (&'a str, Vector) {
        let request = CreateEmbeddingRequest {
            model: "text-embedding-3-large".to_string(),
            input: text.into(),
            dimensions: Some(DIMENSIONS),
            ..Default::default()
        };

        let response = self.client.embeddings().create(request).await.unwrap();

        (text, Vector(response.data[0].embedding.clone()))
    }

    pub async fn new() -> Self {
        let client = Box::new(Client::new());
        Embedder {
            client: Box::leak(client),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_embedding() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let embeddings = Embedder::new().await;
            let result = embeddings.embeddings(vec!["test text"]).await;
            println!("Length of embedding vector: {}", result[0].0.len());
            assert!(!result[0].0.is_empty(), "Embedding should not be empty");
        });
    }

    #[test]
    fn test_similarity() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let embeddings = Embedder::new().await;
            let queries = vec![
                "George Washington",
                "John Adams",
                "Thomas Jefferson",
                "James Madison",
                "James Monroe",
                "John Quincy Adams",
                "Andrew Jackson",
                "Martin Van Buren",
                "William Henry Harrison",
                "John Tyler",
                "vignesh peddi",
                "david xiong",
                "jeremy kuo",
                "computer",
                "dog",
                "cat",
                "herman pewlite",
                "michelle obama",
            ];

            let query_embeddings = embeddings.embeddings(queries.clone()).await;
            let search_text = "intellegence";
            let search_embedding = embeddings.embeddings(vec![search_text]).await;

            // printn similarities
            for (query, embedding) in query_embeddings {
                let similarity = cosine_similarity(&search_embedding[0].1 .0, &embedding.0);
                println!("Query: {:?} Similarity: {}", query, similarity);
            }
        });
    }
}

fn cosine_similarity(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    let dot_product: f32 = vec1.iter().zip(vec2).map(|(a, b)| a * b).sum();
    let magnitude1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();
    let similarity = dot_product / (magnitude1 * magnitude2);

    // High similarity score: 0.8 - 1.0
    // Medium similarity score: 0.5 - 0.8
    similarity
}
