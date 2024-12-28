use std::{path::{Path, PathBuf}, sync::Arc};

use db::FileKey;
use embedder::{Embeddable, Embedder};
use md_parser::Document;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use itertools::Itertools;
use tracing::{info_span, instrument, Instrument};
mod db;
mod embedder;

type VaultItem = (Entity, Option<embedder::Embedding>);
type VaultDB = db::FileDB<VaultItem>;

pub struct Vault {
    root_dir: &'static Path,
    db: VaultDB,
    embedder: Embedder,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Entity {
    File { key: Arc<FileKey>, content: String },
    Heading { key: Arc<FileKey>, content: String },
}

type Score = f32;

type VaultSync<T> = db::Sync<T, VaultItem>;

impl Vault {
    /// Find the k best matches for a query embedding
    #[instrument(skip(self, query_embedding))]
    pub fn best_matches(&self, query_embedding: &[f32], k: usize) -> anyhow::Result<Vec<(Score, Entity)>> {
        let db_iter = self.db.iter();
        let scores = info_span!("scoring").in_scope(|| db_iter
            .flat_map(|(_, reference)| {
                let value_embedding = reference.1.as_ref()?;

                let similarity: f32 =  {
                    // this works for openai embeddings as they are normalized already

                    // the compiler probably optimizes this to simd.

                    query_embedding
                        .iter()
                        .zip(value_embedding)
                        .map(|(a, b)| a * b)
                        .sum()
                };

                Some((similarity, reference.0.clone()))
            })
            .collect::<Vec<_>>());
        let top_scores = info_span!("reducing scores").in_scope(|| scores.into_iter().k_largest_by_key(k, |(s, _)| OrderedFloat::from(*s)).collect_vec());

        Ok(top_scores)
    }

    pub fn new(root_dir: &'static Path) -> Vault {
        Self {
            root_dir,
            db: VaultDB::new(root_dir),
            embedder: Embedder::new(None),
        }
    }

    /// Search for similar content using a text query
    #[instrument(skip_all)]
    pub async fn search(&self, query: &str, k: usize) -> anyhow::Result<Vec<(Score, Entity)>> {
        let query_embedding = self.embedder.embed_one(query).await?;
        self.best_matches(&query_embedding, k)
    }

    #[instrument(skip(self))]
    pub async fn synced(self) -> anyhow::Result<Self> {
        // create a new msync
        let sync: VaultSync<()> = self.db.new_msync()?;

        // populate the sync with parsed files.
        let parsed: VaultSync<(Arc<FileKey>, md_parser::Document)> = sync
            .async_populate(|file_key, file_content| async move { Document::new(&file_content).map(|it| (file_key.clone(), it)) })
            .await
            .inner_flatten();

        // flat map this into files and headings
        let embeddables: VaultSync<Entity> = parsed.flat_map(|(key, document)| {
            std::iter::once(Entity::File {
                key: key.clone(),
                content: document.content()
            })
            .chain(document.sections().flat_map(|it| {
                it.heading.as_ref().map(|_| Entity::Heading {
                        key: key.clone(),
                    content: it.content(),
                })
            }))
            //.chain(document.all_doc_blocks().map(|blcok| block))
            .collect::<Vec<_>>()
        });

        let embedded_syncer: VaultSync<(Entity, Option<embedder::Embedding>)> = embeddables
            .external_async_map(|embeddables| async {
                if !embeddables.is_empty() {
                    self.embedder.embed(embeddables).await
                } else {
                    Ok(Default::default())
                }
            })
            .await?;

        let effect: anyhow::Result<VaultDB> = embedded_syncer.run().await;
        let updated: VaultDB = effect?;

        Ok(Self {
            db: updated,
            ..self
        })
    }
}

impl Embeddable for Entity {
    fn content(&self) -> String {
        match self {
            Entity::File { content, .. } => content.to_string(),
            Entity::Heading { content, .. } => content.to_string(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tracing_subscriber::{
        fmt::{self, format::FmtSpan},
        EnvFilter,
    };

    #[tokio::test]
    async fn test_search() -> anyhow::Result<()> {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::CLOSE)
            .init();

        let test_files_dir = PathBuf::from("/home/felix/notes");
        tracing::info!("Test files directory: {:?}", test_files_dir);

        let vault = Vault::new(Box::leak(test_files_dir.into_boxed_path()));
        let vault = vault.synced().await?;
        tracing::info!("Vault synced successfully");

        let query = "notes about overlaps between Oxide and Exa";
        tracing::info!("Searching for: {}", query);
        let results = vault.search(query, 10).await?;
        
        tracing::debug!("Search results:");
        for (i, (score, entity)) in results.iter().enumerate() {
            let (kind, content) = match entity {
                Entity::File { content, key } => (format!("Key: {key}; File"), content.chars().take(800).collect::<String>()),
                Entity::Heading { content, key  } => (format!("Key: {key}: Heading"), content.to_string()),
            };
            tracing::debug!(
                "\n{}: {}\n   Score: {:.3}\n   Content: {}...", 
                i + 1,
                kind,
                score, 
                content.replace('\n', " ").trim()
            );
        }

        // Verify results
        assert!(!results.is_empty(), "Should return at least one result");
        assert!(results.len() <= 10, "Should not return more than k results");

        // Verify scores are in descending order
        for window in results.windows(2) {
            assert!(
                window[0].0 >= window[1].0, 
                "Results should be in descending order: {} >= {}", 
                window[0].0, 
                window[1].0
            );
        }

        // Verify all scores are valid
        for (score, _) in &results {
            assert!(*score >= 0.0 && *score <= 1.0, "Score should be between 0 and 1: {}", score);
        }

        Ok(())
    }
}
