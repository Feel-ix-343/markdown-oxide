use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use anyhow::anyhow;
use embedder::Embeddable;
use index::Index;
use itertools::Itertools;
use md::ParsedFile;
use mem_fs::{MemFS, Snapshot};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};
use tracing::{debug, error, info, instrument};

impl Vault {
    /// Initializes, updates, removes
    #[instrument(skip(self))]
    pub async fn sync(&self) -> anyhow::Result<()> {
        // memfs sync
        let snapshot = self.mem_fs.read().await?;

        let paths = self
            .index
            .map(|relative_path, value| Ok((relative_path.clone().into_owned(), value.0)))?
            .into_iter()
            .flatten() // flatten the option
            .map(|(path, time)| {
                let full_path = format!("{}/{path}", self.root_dir.to_str().unwrap());
                (full_path, time)
            })
            .collect::<HashSet<_>>();

        let modified_paths = snapshot
            .iter()
            .filter(|(path, (_rope, time))| {
                let kv = (path.to_str().unwrap().to_string(), *time);
                !paths.contains(&kv)
            })
            .collect::<Vec<_>>();

        info!("Num Modified paths: {:?}", modified_paths.len());

        let parsed_files = modified_paths
            .into_iter()
            .flat_map(|(path, (rope, last_modified))| {
                anyhow::Ok((
                    path.clone(),
                    last_modified,
                    md::ParsedFile::construct(&path, rope).map_err(|e| {
                        error!("Failed to consturct document: {e:?}");
                        e
                    })?,
                ))
            })
            .collect::<Vec<_>>();

        if parsed_files.is_empty() {
            info!("No new files to embed");
            return Ok(());
        }

        let entities = parsed_files
            .into_iter()
            .flat_map(|(path, last_modified, parsed_file)| {
                let ParsedFile(file, headings, blocks) = parsed_file;
                std::iter::once(entity::EntityObject::from_file(
                    file.clone(),
                    path.clone(),
                    snapshot.clone(),
                    *last_modified,
                ))
                .chain(headings.into_iter().map(|heading| {
                    entity::EntityObject::from_heading(
                        heading.clone(),
                        path.clone(),
                        snapshot.clone(),
                        *last_modified,
                    )
                }))
                .chain(blocks.into_iter().map(|block| {
                    entity::EntityObject::from_block(
                        block.clone(),
                        path.clone(),
                        snapshot.clone(),
                        *last_modified,
                    )
                }))
                .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let embeddings = self
            .embedder
            .embed(&entities.iter().collect::<Vec<_>>())
            .await?;

        let organized_arcs = entities.into_iter().flat_map(move |entity| {
            let embedding = embeddings
                .get(&entity.id())
                .ok_or(anyhow!("Embedding should exist"))?
                .clone();
            let path = entity.path();
            anyhow::Ok((path, embedding, entity))
        });

        let values = organized_arcs
            .into_group_map_by(|i| i.0.clone())
            .into_iter()
            .map(|(key, value)| {
                let last_modified = snapshot.get(&key).unwrap().1;
                let (file, headings, blocks) = value.into_iter().fold(
                    (None, Vec::new(), Vec::new()),
                    |(mut file, mut headings, mut blocks), (_, embedding, entity)| {
                        let embedding =
                            Arc::into_inner(embedding).expect("should be last embedding reference");
                        match entity {
                            entity::EntityObject::File(f) => {
                                file = Some((
                                    f.into_inner().expect("should be last reference"),
                                    embedding,
                                ))
                            }
                            entity::EntityObject::Heading(h) => headings.push((
                                h.into_inner().expect("should be last reference"),
                                embedding,
                            )),
                            entity::EntityObject::Block(b) => blocks.push((
                                b.into_inner().expect("should be last reference"),
                                embedding,
                            )),
                        };
                        (file, headings, blocks)
                    },
                );

                let relative_path = std::borrow::Cow::Owned(
                    key.strip_prefix(self.root_dir)?
                        .to_str()
                        .ok_or(anyhow!("Failed to convert to string"))?
                        .to_owned(),
                );

                info!("Relative Path: {:?}", relative_path);

                Ok((
                    relative_path,
                    Some(Value(last_modified, file, headings, blocks)),
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        self.index.sync_files(values).await?;

        Ok(())
    }

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

    #[instrument(skip(self))]
    pub async fn embeddings_search(
        &self,
        query: &str,
    ) -> anyhow::Result<Vec<(f32, entity::EntityObject)>> {
        let query_embedding = self.embedder.embed_query(query).await?;

        let snapshot: Snapshot = self.mem_fs.read().await?;

        let all_entities_and_embeddings = self
            .index
            .map(|key, value| {
                let path = PathBuf::from(format!(
                    "{}/{}",
                    self.root_dir.to_str().unwrap(),
                    key.to_owned()
                ));
                let Value(last_modified, file, headings, blocks) = value;

                let mut entities_and_embeddings = Vec::new();

                if let Some((f, embedding)) = file {
                    entities_and_embeddings.push((
                        entity::EntityObject::from_file(
                            Arc::new(f),
                            Arc::from(path.clone()),
                            snapshot.clone(),
                            last_modified,
                        ),
                        embedding,
                    ));
                }

                entities_and_embeddings.extend(headings.into_iter().map(|(h, embedding)| {
                    (
                        entity::EntityObject::from_heading(
                            Arc::new(h),
                            Arc::from(path.clone()),
                            snapshot.clone(),
                            last_modified,
                        ),
                        embedding,
                    )
                }));

                entities_and_embeddings.extend(blocks.into_iter().map(|(b, embedding)| {
                    (
                        entity::EntityObject::from_block(
                            Arc::new(b),
                            Arc::from(path.clone()),
                            snapshot.clone(),
                            last_modified,
                        ),
                        embedding,
                    )
                }));

                Ok(entities_and_embeddings)
            })?
            .unwrap()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let mut entity_scores: Vec<(f32, entity::EntityObject)> = all_entities_and_embeddings
            .into_par_iter()
            .map(|(entity, embedding)| {
                let similarity = Self::cosine_similarity(&query_embedding.0, &embedding.0);
                (similarity, entity)
            })
            .collect();

        entity_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        Ok(entity_scores.into_iter().collect())
    }

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

#[derive(Serialize, Deserialize, Debug)]
struct Value(
    SystemTime, // last modified
    Option<(md::File, embedder::Embedding)>,
    Vec<(md::Heading, embedder::Embedding)>,
    Vec<(md::Block, embedder::Embedding)>,
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
            let results = vault.embeddings_search(query).await.unwrap();

            println!("Top results for query: '{}'", query);
            for (i, (score, result)) in results.iter().enumerate().take(10) {
                println!("{}. score: {}", i + 1, score);
                match result {
                    entity::EntityObject::File(f) => {
                        println!("{}. file {:?}", i + 1, f.path(),);
                    }
                    entity::EntityObject::Heading(h) => {
                        println!("{}. {:?} heading {:?}", i + 1, h.path(), h.heading_name());
                    }
                    entity::EntityObject::Block(b) => {
                        println!("{}. block {:?}", i + 1, b.path(),);
                    }
                }

                println!("{:?}", result.content())
            }

            // Add more specific assertions here if needed, e.g.:
            // assert!(results[0].content().unwrap().contains(query), "Top result should contain the query");
        }
    }
}
