use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use anyhow::anyhow;
use embedder::{Embeddable, EmbeddableStructure, Embedding};
use index::Index;
use itertools::Itertools;
use mem_fs::{MemFS, Metadata, Snapshot};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};
use tracing::{debug, error, info, instrument};

impl Vault {
    // pub fn semantic_search(&self, query: &str) -> _ {
    //     todo!()
    // }

    // pub fn similarity_search(&self, entity_reference: E) -> _ {
    //     todo!()
    // }
    #[instrument]
    pub async fn init(root_dir: &'static Path) -> Result<Self> {
        let index = Index::new(&root_dir.join("oxide.db"))
            .await
            .context("Failed to create index")?;
        let mem_fs = MemFS::new(root_dir).context("Failed to create MemFS")?;
        let embedder = embedder::Embedder::new();
        Ok(Vault {
            index,
            mem_fs,
            root_dir,
            embedder,
        })
    }

    // #[instrument(skip(self))]
    // pub async fn embeddings_search(
    //     &self,
    //     query: &str,
    // ) -> anyhow::Result<Vec<(f32, entity::EntityObject)>> {
    //     let query_embedding = self.embedder.embed_query(query).await?;

    //     let snapshot: Snapshot = self.mem_fs.read().await?;

    //     let all_entities_and_embeddings = self
    //         .index
    //         .map(|key, value| {
    //             let path = PathBuf::from(format!(
    //                 "{}/{}",
    //                 self.root_dir.to_str().unwrap(),
    //                 key.to_owned()
    //             ));
    //             let Value(last_modified, file, headings, blocks) = value;

    //             let mut entities_and_embeddings = Vec::new();

    //             if let Some((f, embedding)) = file {
    //                 entities_and_embeddings.push((
    //                     entity::EntityObject::from_file(
    //                         Arc::new(f),
    //                         Arc::from(path.clone()),
    //                         snapshot.clone(),
    //                         last_modified,
    //                     ),
    //                     embedding,
    //                 ));
    //             }

    //             entities_and_embeddings.extend(headings.into_iter().map(|(h, embedding)| {
    //                 (
    //                     entity::EntityObject::from_heading(
    //                         Arc::new(h),
    //                         Arc::from(path.clone()),
    //                         snapshot.clone(),
    //                         last_modified,
    //                     ),
    //                     embedding,
    //                 )
    //             }));

    //             entities_and_embeddings.extend(blocks.into_iter().map(|(b, embedding)| {
    //                 (
    //                     entity::EntityObject::from_block(
    //                         Arc::new(b),
    //                         Arc::from(path.clone()),
    //                         snapshot.clone(),
    //                         last_modified,
    //                     ),
    //                     embedding,
    //                 )
    //             }));

    //             Ok(entities_and_embeddings)
    //         })?
    //         .unwrap()
    //         .into_iter()
    //         .flatten()
    //         .collect::<Vec<_>>();

    //     let mut entity_scores: Vec<(f32, entity::EntityObject)> = all_entities_and_embeddings
    //         .into_par_iter()
    //         .map(|(entity, embedding)| {
    //             let similarity = Self::cosine_similarity(&query_embedding.0, &embedding.0);
    //             (similarity, entity)
    //         })
    //         .collect();

    //     entity_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    //     Ok(entity_scores.into_iter().collect())
    // }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot_product / (magnitude_a * magnitude_b)
    }
}

pub struct Vault {
    index: index::Index<Value>,
    mem_fs: MemFS,
    root_dir: &'static Path,
    embedder: embedder::Embedder,
}

impl Vault {
    /// Initializes, updates, removes
    #[instrument(skip(self))]
    pub async fn sync(&self) -> anyhow::Result<()> {
        // memfs sync
        let snapshot: Snapshot = self.mem_fs.read().await?;

        let indexed_files: HashSet<(String, SystemTime)> = self
            .index
            .map(|relative_path, value| {
                Ok((relative_path.clone().into_owned(), value.0.last_modified))
            })?
            .into_iter()
            .flatten() // flatten the option
            .collect::<HashSet<_>>();

        // INdex update actions
        let index_updates = snapshot
            .iter()
            .filter(|(relative_path, (metadata, _))| {
                let file_state = (relative_path.to_string(), metadata.last_modified); // optimize if necessary
                !indexed_files.contains(&file_state)
            })
            .collect::<Vec<_>>();

        info!("Num Modified paths: {:?}", index_updates.len());

        let parsed_files = index_updates
            .into_iter()
            .filter(|(_, (_, file))| !file.text().is_empty())
            .map(|(path, (meta, file))| {
                anyhow::Ok((
                    path,
                    meta,
                    md::parse_file(path, file).map_err(|e| {
                        error!("Failed to consturct document: {e:?}");
                        e
                    })?,
                ))
            })
            .flatten();

        let structures = parsed_files
            .map(|(path, metadata, (file, headings, blocks))| {
                (
                    file,
                    headings,
                    blocks,
                    path.as_str(),
                    snapshot.clone(),
                    metadata.clone(), // optimize this if necessary
                )
            })
            .collect::<Vec<_>>();

        let embedded_structures = self.embedder.embed_structures(structures).await?;

        self.index
            .sync_files(
                embedded_structures
                    .into_iter()
                    .map(|(path, value)| (path, Some(value)))
                    .collect(),
            )
            .await?;

        Ok(())
    }
}

impl<'a> EmbeddableStructure<(&'a str, Value)>
    for (
        md::File,
        Vec<md::Heading>,
        Vec<md::Block>,
        &'a str,
        Snapshot,
        Metadata,
    )
{
    fn into(self, embeddings: Vec<anyhow::Result<Embedding>>) -> (&'a str, Value) {
        let (file, headings, blocks, path, _snapshot, metadata) = self;
        let mut embedding_iter = embeddings.into_iter();

        let file_embedding = embedding_iter
            .next()
            .map(|e| {
                e.map_err(|err| {
                    error!("Error embedding file: {:?}; Relative Path: {:?}", err, path);
                    anyhow::anyhow!("Failed to embed file '{}': {}", path, err)
                })
                .ok()
            })
            .flatten();

        let heading_embeddings = headings
            .into_iter()
            .zip(embedding_iter.by_ref())
            .map(|(heading, embedding)| {
                let title = heading.title.clone();
                (
                    heading,
                    embedding
                        .map_err(|err| {
                            error!(
                                "Error embedding heading '{}': {:?}; Relative Path: {:?}",
                                title, err, path
                            );
                            err
                        })
                        .ok(),
                )
            })
            .collect();

        let block_embeddings = blocks
            .into_iter()
            .zip(embedding_iter)
            .map(|(block, embedding)| {
                let block_display = format!("{:?}", block);
                (
                    block,
                    embedding
                        .map_err(|err| {
                            error!("Error embedding block: {:?}; Relative Path {:?}; block: {block_display}", err, path);
                            err
                        })
                        .ok(),
                )
            })
            .collect();

        (
            path,
            Value(
                metadata,
                (file, file_embedding),
                heading_embeddings,
                block_embeddings,
            ),
        )
    }

    fn into_content(&self) -> Vec<anyhow::Result<String>> {
        let (file, headings, blocks, relative_path, snapshot, ..) = self;
        let entities = std::iter::once(entity::EntityObject::from_file(
            file,
            relative_path,
            snapshot.clone(),
            SystemTime::now(),
        ))
        .chain(headings.iter().map(|heading| {
            entity::EntityObject::from_heading(
                heading,
                relative_path,
                snapshot.clone(),
                SystemTime::now(),
            )
        }))
        .chain(blocks.iter().map(|block| {
            entity::EntityObject::from_block(
                block,
                relative_path,
                snapshot.clone(),
                SystemTime::now(),
            )
        }));

        entities
            .map(|entity| entity.entity_content().map(|it| it.into_owned()))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Value(
    Metadata, // last modified
    (md::File, Option<embedder::Embedding>),
    Vec<(md::Heading, Option<embedder::Embedding>)>,
    Vec<(md::Block, Option<embedder::Embedding>)>,
);

impl index::IndexValue for Value {
    fn as_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn from_bytes(bytes: &[u8]) -> Self
    where
        Self: Sized,
    {
        bincode::deserialize(bytes).unwrap()
    }
}

mod embedder;
mod entity;
mod index;
mod md;
mod mem_fs;

#[cfg(test)]
mod tests {
    use entity::EntityObjectInterface;
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_vault_initialization() {
        tracing_subscriber::fmt()
            .with_file(true)
            .with_line_number(true)
            .with_env_filter(
                EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
            )
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .init();
        let root_dir = Box::leak(Box::new(PathBuf::from("/home/felix/notes")));
        let vault = Vault::init(root_dir).await.unwrap();

        assert_eq!(vault.root_dir, root_dir);
        assert!(vault.mem_fs.read().await.is_ok());
        // Additional assertions can be added here to check other properties of the initialized Vault

        // Test synchronization
        vault.sync().await.unwrap();
    }

    #[tokio::test]
    async fn test_embeddings_search() {
        tracing_subscriber::fmt()
            .with_file(true)
            .with_line_number(true)
            .with_env_filter(
                EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
            )
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .init();
        let root_dir = Box::leak(Box::new(PathBuf::from("/home/felix/notes")));
        let vault = Vault::init(root_dir).await.unwrap();

        // Ensure the vault is synced before searching
        // vault.sync().await.unwrap();

        let queries = vec![
            "Software Engineering",
            "My personal perspective on drinking coffee",
            "Thoughts about programming with the aid of LLMs",
            "Thoughts about the purpose of Exa",
            "high inflation and economic crisis in latin america",
            "My notes on modern african civilizations",
            "continuities and changes in russia throughout history",
            "fundemental principals of government",
            "- The issue you have noticed for the past for months is a wavering motivation, which was different than that of the months prior, in developing oxide initial version.
                * the diagnosis is that you did not in fact accomplish anything that whole time."
        ];

        for query in queries {
            // let results = vault.embeddings_search(query).await.unwrap();

            // println!("Top results for query: '{}'", query);
            // for (i, (score, result)) in results.iter().enumerate().take(10) {
            //     println!("{}. score: {}", i + 1, score);
            //     match result {
            //         entity::EntityObject::File(f) => {
            //             println!("{}. file {:?}", i + 1, f.path(),);
            //         }
            //         entity::EntityObject::Heading(h) => {
            //             println!("{}. {:?} heading {:?}", i + 1, h.path(), h.heading_name());
            //         }
            //         entity::EntityObject::Block(b) => {
            //             println!("{}. block {:?}", i + 1, b.path(),);
            //         }
            //     }

            //     // println!("{:?}", result.content())
            // }

            // Add more specific assertions here if needed, e.g.:
            // assert!(results[0].content().unwrap().contains(query), "Top result should contain the query");
        }
    }
}
