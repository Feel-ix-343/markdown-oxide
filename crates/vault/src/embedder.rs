use std::collections::HashMap;

use anyhow::anyhow;
use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequest};
use futures::{stream, StreamExt};
use itertools::Itertools;
use tiktoken_rs::cl100k_base;
use tracing::{info, instrument, span, Level};

pub type Embedding = Vec<f32>;

pub struct Embedder {
    client: async_openai::Client<OpenAIConfig>,
}

pub trait Embeddable {
    fn content(&self) -> String;
}

use std::env;

impl Embedder {
    #[instrument]
    pub fn new(key: Option<&str>) -> Self {
        let api_key = key
            .map(String::from)
            .or_else(|| env::var("OPENAI_API_KEY").ok());

        let config = match api_key {
            Some(key) => OpenAIConfig::new().with_api_key(key),
            None => OpenAIConfig::new(), // This will use the default configuration
        };

        Self {
            client: async_openai::Client::with_config(config),
        }
    }

    /// Embeds the items and returns them in the same order passed in.
    #[instrument(skip(self, embeddables))]
    pub async fn embed<I: Embeddable + std::fmt::Debug>(
        &self,
        embeddables: Vec<I>,
    ) -> anyhow::Result<Vec<(I, Option<Embedding>)>> {
        let len = embeddables.len();
        info!("Embedding {len} items");
        let num_chunks = len / 2048 + 1;
        let embeddables = embeddables.into_iter().enumerate();

        let r = stream::iter(embeddables.chunks(2048).into_iter().enumerate())
            .map(|(idx, it)| async move {
                let span = span!(Level::INFO, "Embeddings request", idx);
                let _ = span.enter();

                let validated_content = it
                    .map(|item| {
                        let content = item.1.content();
                        (item, content)
                    })
                    .map(|(item, content)| {
                        if content.is_empty() {
                            info!("Content for item {:?} empty; not embedding", item.1);
                            (item, None)
                        } else {
                            (item, Some(content))
                        }
                    })
                    .collect::<Vec<_>>();

                let valid_content = validated_content
                    .iter()
                    .flat_map(|(item, content)| content.as_ref().map(|content| (item, content)))
                    .map(|it| (it.0 .0, it.1))
                    .map(|(idx, string)| -> (usize, String) {
                        let bpe = cl100k_base().unwrap();
                        let tokens = bpe.encode_with_special_tokens(&string);
                        let truncated_tokens =
                            tokens.iter().take(8192).cloned().collect::<Vec<_>>();
                        let truncated_string = bpe.decode(truncated_tokens).unwrap();
                        println!("Tokenized item {}: {} tokens", idx, tokens.len());
                        (idx, truncated_string)
                    });

                let text = valid_content
                    .clone()
                    .map(|(_idx, string)| string)
                    .collect::<Vec<_>>();

                info!("Sending embeddings request {}/{}", idx + 1, num_chunks);

                let embeddings = self
                    .client
                    .embeddings()
                    .create(CreateEmbeddingRequest {
                        model: "text-embedding-3-large".to_string(),
                        input: text.into(),
                        user: None,
                        encoding_format: None,
                        dimensions: None,
                    })
                    .await?
                    .data
                    .into_iter()
                    .map(|it| it.embedding)
                    .collect::<Vec<Embedding>>();

                info!("Recieved embeddings response {}/{}", idx + 1, num_chunks);

                let embeddings_map = valid_content
                    .zip(embeddings)
                    .map(|it| (it.0 .0, it.1))
                    .collect::<HashMap<_, _>>();

                let results = validated_content
                    .into_iter()
                    .map(|((idx, item), validated_content)| match validated_content {
                        Some(_) => {
                            let embedding = embeddings_map.get(&idx).ok_or(anyhow!(
                                "Failed to get the embedding for validted item {item:?}"
                            ))?;
                            Ok((idx, item, Some(embedding.to_owned())))
                        }
                        None => Ok((idx, item, None)),
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;

                anyhow::Ok(results)
            })
            .buffer_unordered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .sorted_by_key(|it| it.0)
            .map(|it| (it.1, it.2))
            .collect_vec();

        Ok(r)
    }
}
