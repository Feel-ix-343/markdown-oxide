use std::{borrow::Cow, collections::HashMap, hash::Hash, ops::Not, sync::Arc};

use async_openai::config::{Config, OpenAIConfig};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tiktoken_rs::tokenizer::Tokenizer;
use tracing::{error, info, instrument};

use crate::entity;

#[derive(Serialize, Deserialize, Debug)]
pub struct Embedding(Vec<f32>);

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

                info!("Getting contents");
                let contents = chunk
                    .par_iter()
                    .map(|item| {
                        anyhow::Ok((
                            item.id(),
                            item.content().map_err(|e| {
                                error!("Failed to get content: {e:?}");
                                e
                            })?,
                        ))
                    })
                    .flatten()
                    .map(|(id, content)| {
                        let tokens = self.tokenizer.encode_ordinary(&content);
                        if tokens.len() > 8192 {
                            info!("{id:?}: Truncating content to 8192 tokens");
                        }
                        let truncated_tokens = tokens.into_iter().take(8192).collect::<Vec<_>>();
                        (id, self.tokenizer.decode(truncated_tokens).unwrap())
                    })
                    .filter(|(_, content)| !content.is_empty())
                    .collect::<Vec<_>>();

                info!("Making request");
                let request = async_openai::types::CreateEmbeddingRequestArgs::default()
                    .model("text-embedding-3-small")
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
                    chunk
                        .iter()
                        .zip(response.data.into_iter())
                        .map(|(item, embedding)| {
                            (item.id(), Arc::new(Embedding(embedding.embedding)))
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .buffer_unordered(3)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect())
    }
}
