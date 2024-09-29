use std::{borrow::Cow, collections::HashMap, hash::Hash, ops::Not, sync::Arc};

use async_openai::config::{Config, OpenAIConfig};
use derive_deref::Deref;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tiktoken_rs::tokenizer::Tokenizer;
use tracing::{error, info, instrument};

use crate::entity;

#[derive(Serialize, Deserialize, Debug, Deref)]
pub struct Embedding(pub Vec<f32>);

pub trait Embeddable<Id: Eq + Hash> {
    fn content(&self) -> anyhow::Result<Cow<str>>;
    fn id(&self) -> Id;
}

#[derive(Debug)]
pub struct Embedder {
    openai_client: async_openai::Client<OpenAIConfig>,
    tokenizer: tiktoken_rs::CoreBPE,
}

use std::fmt::Debug;
impl Embedder {
    /// Reads from the OPENAI_API_KEY env var to construct
    #[instrument]
    pub fn new() -> Self {
        let config = async_openai::config::OpenAIConfig::new();
        let openai_client = async_openai::Client::with_config(config);
        let tokenizer = tiktoken_rs::get_bpe_from_tokenizer(Tokenizer::Cl100kBase).unwrap();
        Embedder {
            openai_client,
            tokenizer,
        }
    }

    #[instrument(skip(self))]
    pub async fn embed_query(&self, query: &str) -> anyhow::Result<Embedding> {
        let request = async_openai::types::CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-large")
            .input(query)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build embedding request: {}", e))?;

        let response = self.openai_client.embeddings().create(request).await?;

        if response.data.is_empty() {
            return Err(anyhow::anyhow!("No embedding returned from OpenAI"));
        }

        Ok(Embedding(response.data[0].embedding.clone()))
    }

    #[instrument(skip(self, items))]
    pub async fn embed<Id: Send + Eq + Hash + Debug, E: Embeddable<Id> + Sync>(
        &self,
        items: &[&E],
    ) -> anyhow::Result<HashMap<Id, Arc<Embedding>>> {
        use futures::stream::{self, StreamExt};

        let chunks: Vec<_> = items.chunks(2048).collect();
        let total_chunks = chunks.len();
        Ok(stream::iter(chunks.into_iter().enumerate())
            .map(|(i, chunk)| async move {
                info!(
                    "Embedding chunk {}/{total_chunks} of {} items",
                    i + 1,
                    chunk.len()
                );

                // filtered chunk
                let filtered_chunks = chunk
                    .par_iter()
                    .map(|chunk| anyhow::Ok((chunk, chunk.content()?)))
                    .flatten()
                    .filter(|(_, content)| !content.is_empty())
                    .collect::<Vec<_>>();

                info!("Getting contents");
                let contents = filtered_chunks
                    .par_iter()
                    .map(|(item, content)| (item.id(), content))
                    .map(|(id, content)| {
                        let tokens = self.tokenizer.encode_ordinary(&content);
                        if tokens.len() > 8192 {
                            info!("{id:?}: Truncating content to 8192 tokens");
                        }
                        let truncated_tokens = tokens.into_iter().take(8192).collect::<Vec<_>>();
                        (id, self.tokenizer.decode(truncated_tokens).unwrap())
                    })
                    .collect::<Vec<_>>();

                info!("Making request");
                let request = async_openai::types::CreateEmbeddingRequestArgs::default()
                    .model("text-embedding-3-large")
                    .input(
                        contents
                            .iter()
                            .map(|(_, content)| content.as_str())
                            .collect::<Vec<_>>(),
                    )
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build embedding request: {}", e))?;

                let response = self.openai_client.embeddings().create(request).await?;

                anyhow::Ok(
                    filtered_chunks
                        .iter()
                        .map(|(item, _)| item)
                        .zip(response.data.into_iter())
                        .map(|(item, embedding)| {
                            (item.id(), Arc::new(Embedding(embedding.embedding)))
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .buffer_unordered(5)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect())
    }
}
