use std::{path::Path, sync::Arc};

use anyhow::anyhow;
use embedder::Embeddable;
use entity::EntityObject;
use index::{Index, IndexValue};
use itertools::Itertools;
use md::ParsedFile;
use mem_fs::MemFS;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use anyhow::{Context, Result};
use tracing::instrument;

impl Vault {
    /// Initializes, updates, removes
    pub async fn sync(&self) -> anyhow::Result<()> {
        let snapshot = self.mem_fs.read().await?;
        let parsed_files = snapshot
            .iter()
            .map(|(path, (rope, document))| {
                Ok((
                    path.clone(),
                    md::ParsedFile::construct(&path, rope, document)?,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let entities = parsed_files
            .into_iter()
            .map(|(path, parsed_file)| {
                let ParsedFile(file, headings, blocks) = parsed_file;
                std::iter::once(entity::EntityObject::from_file(
                    file.clone(),
                    path.clone(),
                    snapshot.clone(),
                ))
                .chain(headings.into_iter().map(|heading| {
                    entity::EntityObject::from_heading(
                        heading.clone(),
                        path.clone(),
                        snapshot.clone(),
                    )
                }))
                .chain(blocks.into_iter().map(|block| {
                    entity::EntityObject::from_block(block.clone(), path.clone(), snapshot.clone())
                }))
                .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        let embeddings = self
            .embedder
            .embed(&entities.iter().collect::<Vec<_>>())
            .await?;

        let organized_arcs = entities
            .into_iter()
            .map(move |entity| {
                let embedding = embeddings
                    .get(&entity.id())
                    .ok_or(anyhow!("Embedding should exist"))?
                    .clone();
                let path = entity.path();
                anyhow::Ok((path, embedding, entity))
            })
            .flatten();

        let values = organized_arcs
            .into_group_map_by(|i| i.0.clone())
            .into_iter()
            .map(|(key, value)| {
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

                Ok((relative_path, Some(Value(file, headings, blocks))))
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
}

pub struct Vault {
    index: index::Index<Value>,
    mem_fs: MemFS,
    root_dir: &'static Path,
    embedder: embedder::Embedder,
}

#[derive(Serialize, Deserialize, Debug)]
struct Value(
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
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter, Layer};

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
}
