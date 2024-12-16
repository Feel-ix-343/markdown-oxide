use std::path::{Path, PathBuf};

use embedder::{Embeddable, Embedder};
use md_parser::Document;
use serde::{Deserialize, Serialize};

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
    File { content: String },
    Heading { content: String },
}

type Score = f64;

type VaultSync<T> = db::Sync<T, VaultItem>;

impl Vault {
    pub fn new(root_dir: &'static Path) -> Vault {
        Self {
            root_dir,
            db: VaultDB::new(root_dir),
            embedder: Embedder::new(None),
        }
    }

    pub async fn synced(self) -> anyhow::Result<Self> {
        // create a new msync
        let sync: VaultSync<()> = self.db.new_msync().await?;

        // populate the sync with parsed files.
        let parsed: VaultSync<md_parser::Document> = sync
            .async_populate(|_, file_content| async move { Document::new(&file_content) })
            .await
            .inner_flatten();

        // flat map this into files and headings
        let embeddables: VaultSync<Entity> = parsed.flat_map(|document| {
            std::iter::once(Entity::File {
                content: document.rope.to_string(),
            })
            .chain(document.sections().flat_map(|it| {
                it.heading.as_ref().map(|_| Entity::Heading {
                    content: {
                        let range = it.range;
                        let slice = document.rope.byte_slice(range.start_byte..range.end_byte);
                        slice.to_string()
                    },
                })
            }))
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
            Entity::File { content } => content.to_string(),
            Entity::Heading { content } => content.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tracing::info;
    use tracing_subscriber::{
        fmt::{self, format::FmtSpan},
        EnvFilter,
    };

    #[tokio::test]
    async fn test_vault_synced() -> anyhow::Result<()> {
        // Initialize tracing subscriber
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::CLOSE)
            .init();

        // Get the project directory using Cargo's CARGO_MANIFEST_DIR environment variable
        //let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        //let test_files_dir = project_dir.join("../../TestFiles");

        let test_files_dir = PathBuf::from("/home/felix/notes");

        tracing::info!("Test files directory: {:?}", test_files_dir);

        // Create a new Vault instance with the TestFiles directory
        let vault = Vault::new(Box::leak(test_files_dir.into_boxed_path()));

        tracing::info!("Created Vault instance");

        // Sync the vault
        let synced_vault = vault.synced().await?;

        tracing::info!("Vault synced successfully");

        let it = synced_vault.db.map(|file, value| {
            (
                file.to_owned(),
                value.0.clone(),
                value
                    .1
                    .as_ref()
                    .and_then(|it| it.get(0..7).map(|it| it.to_vec())),
            )
        })?;

        info!("Test results {:?}", it.iter().take(10).collect::<Vec<_>>());

        assert!(it.len() != 0);

        Ok(())
    }
}
