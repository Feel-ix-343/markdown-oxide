trait Embedder {}

trait Data<T: Embedder> {}

pub struct Engine<T: Embedder>(T);

pub struct Embedding<T: Embedder>();

impl<T: Embedder> Engine<T> {
    /// One to one mapping of data to embedding result; indexes correspond.
    pub async fn embed_many<D: Data<T>>(&self, data: &[&D]) -> Vec<anyhow::Result<Embedding<T>>> {
        self.0.embed(data)
    }

    pub async fn embed<D: Data<T>>(&self, data: &D) -> anyhow::Result<Embedding<T>> {
        self.0.embed(&[data]).await
    }
}

trait EmbeddableData<T: Embedder> {}

pub trait EmbeddableStructure<T: Embedder, D: Data<T>> {
    fn to_data(&self) -> anyhow::Result<Vec<D>>;
}

impl<T: Embedder> Engine<T> {
    pub async fn embed_structures<D: Data<T>, S: EmbeddableStructure<T, Data<T>>>(
        &self,
        structures: &[&S],
    ) -> Vec<anyhow::Result<Embedding<T>>> {
        todo!()
    }
}

// use std::{borrow::Cow, collections::HashMap, hash::Hash, ops::Not, sync::Arc};
//
// use anyhow::anyhow;
// use async_openai::config::{Config, OpenAIConfig};
// use derive_deref::Deref;
// use itertools::Itertools;
// use rayon::prelude::*;
// use serde::{Deserialize, Serialize};
// use tiktoken_rs::tokenizer::Tokenizer;
// use tracing::{error, info, instrument};
//
// use crate::entity;

// #[derive(Clone, Serialize, Deserialize, Debug, Deref)]

// pub struct Embedding(pub Vec<f32>);
//
// #[derive(Debug)]
// pub struct Embedder {
//     openai_client: async_openai::Client<OpenAIConfig>,
//     tokenizer: tiktoken_rs::CoreBPE,
// }
//
// use std::fmt::Debug;
//
// const OPENAI_MODEL: &str = "text-embedding-3-large";
//
// impl Embedder {
//     /// Reads from the OPENAI_API_KEY env var to construct
//     #[instrument]
//     pub fn new() -> Self {
//         let config = async_openai::config::OpenAIConfig::new();
//         let openai_client = async_openai::Client::with_config(config);
//         let tokenizer = tiktoken_rs::get_bpe_from_tokenizer(Tokenizer::Cl100kBase).unwrap();
//         Embedder {
//             openai_client,
//             tokenizer,
//         }
//     }
//
//     #[instrument(skip(self))]
//     pub async fn embed_query(&self, query: &str) -> anyhow::Result<Embedding> {
//         let request = async_openai::types::CreateEmbeddingRequestArgs::default()
//             .model(OPENAI_MODEL)
//             .input(query)
//             .build()
//             .map_err(|e| anyhow::anyhow!("Failed to build embedding request: {}", e))?;
//
//         let response = self.openai_client.embeddings().create(request).await?;
//
//         if response.data.is_empty() {
//             return Err(anyhow::anyhow!("No embedding returned from OpenAI"));
//         }
//
//         Ok(Embedding(response.data[0].embedding.clone()))
//     }
// }
//
// pub trait Embeddable {
//     fn content(&self) -> anyhow::Result<String>;
// }
//
// impl Embedder {
//     #[instrument(skip(self, items))]
//     /// Returns all embeddings in order
//     pub async fn embed<E: Embeddable + Send + Sync>(
//         &self,
//         items: Vec<E>,
//     ) -> anyhow::Result<Vec<(E, anyhow::Result<Embedding>)>> {
//         use futures::stream::{self, StreamExt};
//
//         // im cheating; optimize this if necessary TODO
//
//         let items = items
//             .into_iter()
//             .map(|item| Arc::new(item))
//             .collect::<Vec<_>>();
//         let chunks: Vec<_> = items.chunks(2048).into_iter().collect();
//         let total_chunks = chunks.len();
//
//         let result_arcs = stream::iter(chunks.into_iter().enumerate())
//             .map(|(i, chunk)| async move {
//                 info!(
//                     "Embedding chunk {}/{total_chunks} of {} items",
//                     i + 1,
//                     chunk.len()
//                 );
//
//                 // filtered chunk
//                 let filtered_embeddables = chunk
//                     .into_par_iter()
//                     .map(|e| (e, e.content()))
//                     .map(|(e, content)| {
//                         (
//                             e,
//                             content.and_then(|content| {
//                                 if content.is_empty() {
//                                     Err(anyhow!("Cannot embed, content empty"))
//                                 } else {
//                                     anyhow::Ok(content)
//                                 }
//                             }),
//                         )
//                     })
//                     .collect::<Vec<_>>();
//
//                 info!("Getting contents");
//                 let contents = filtered_embeddables
//                     .par_iter()
//                     .map(|item| &item.1)
//                     .flatten()
//                     .map(|content| {
//                         let tokens = self.tokenizer.encode_ordinary(&content);
//                         if tokens.len() > 8192 {
//                             info!(
//                                 "Truncating content to 8192 tokens: {:?}...",
//                                 content.get(0..100)
//                             );
//                         }
//                         let truncated_tokens = tokens.into_iter().take(8192).collect::<Vec<_>>();
//                         self.tokenizer.decode(truncated_tokens).unwrap()
//                     })
//                     .collect::<Vec<_>>();
//
//                 info!("Making request");
//                 let request = async_openai::types::CreateEmbeddingRequestArgs::default()
//                     .model(OPENAI_MODEL)
//                     .input(contents)
//                     .build()
//                     .map_err(|e| anyhow::anyhow!("Failed to build embedding request: {}", e))?;
//
//                 let response = self.openai_client.embeddings().create(request).await?;
//
//                 let mut response_data_iter = response.data.into_iter();
//
//                 let embeddings = filtered_embeddables
//                     .into_iter()
//                     .map(|(e, validated_content)| match validated_content {
//                         Ok(_) => {
//                             let embedding = response_data_iter.next();
//                             let embedding = embedding.expect("Next embedding should exist");
//                             (e.clone(), Ok(Embedding(embedding.embedding)))
//                         }
//                         Err(error) => (e.clone(), Err(error)),
//                     })
//                     .collect::<Vec<_>>();
//
//                 Ok(embeddings)
//             })
//             .buffered(3)
//             .collect::<Vec<_>>()
//             .await
//             .into_iter()
//             .collect::<anyhow::Result<Vec<_>>>()?
//             .into_iter()
//             .flatten()
//             .collect::<Vec<_>>();
//
//         drop(items);
//
//         let results = result_arcs
//             .into_iter()
//             .map(|(arc_e, res)| {
//                 let e = Arc::into_inner(arc_e).expect("Failed to unwrap Arc");
//                 (e, res)
//             })
//             .collect::<Vec<_>>();
//
//         Ok(results)
//     }
// }
//
// pub trait EmbeddableStructure<T> {
//     fn into_content(&self) -> Vec<anyhow::Result<String>>;
//     fn into(self, embeddings: Vec<anyhow::Result<Embedding>>) -> T;
// }
//
// impl Embeddable for anyhow::Result<String> {
//     fn content(&self) -> anyhow::Result<String> {
//         match self {
//             Ok(string) => Ok(string.to_string()),
//             Err(e) => Err(anyhow!(e.to_string())), // todo handle this without cloning
//         }
//     }
// }
//
// impl Embedder {
//     /// Constructs a list of embedded structures given a list of structures capable of being embedded
//     #[instrument(skip(self, items))]
//     pub async fn embed_structures<T, F: EmbeddableStructure<T> + Sync>(
//         &self,
//         items: Vec<F>,
//     ) -> anyhow::Result<Vec<T>> {
//         let (structure_contents, lengths): (Vec<_>, Vec<_>) = items
//             .iter()
//             .map(|item| {
//                 let contents = item.into_content();
//                 let length = contents.len();
//                 (contents, length)
//             })
//             .unzip();
//
//         let embeddings = self
//             .embed(structure_contents.into_iter().flatten().collect::<Vec<_>>())
//             .await?
//             .into_iter()
//             .map(|(_, result)| result);
//
//         let mut embeddings_vec = embeddings.collect::<Vec<_>>();
//
//         let grouped_embeddings = lengths
//             .iter()
//             .map(|length| embeddings_vec.drain(..length).collect::<Vec<_>>())
//             .collect::<Vec<_>>();
//
//         Ok(items
//             .into_iter()
//             .zip(grouped_embeddings)
//             .map(|(structure, embeddings)| structure.into(embeddings))
//             .collect())
//     }
// }
