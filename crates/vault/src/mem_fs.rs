use std::{
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context};
use derive_deref::Deref;
use md_parser::Document;
use rayon::iter::{IntoParallelIterator, ParallelBridge};
use ropey::Rope;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, trace};

type MemFSValue = (Rope, std::time::SystemTime);
type MemFSMap = HashMap<Arc<Path>, MemFSValue>;

/// Snapshot of the FS; cheap to clone; safe to cache
#[derive(Debug, Clone, Deref)]
pub struct Snapshot(Arc<MemFSMap>);

impl Snapshot {
    pub fn get<'a>(&'a self, path: &Path) -> anyhow::Result<&'a MemFSValue> {
        self.0.get(path).ok_or(anyhow!("Path doesn't exist"))
    }

    pub fn iter(&self) -> impl Iterator<Item = (Arc<Path>, &MemFSValue)> {
        self.0.iter().map(|(path, value)| (path.clone(), value))
    }
}

pub struct MemFS {
    mem_fs: RwLock<Arc<MemFSMap>>,
}

use rayon::prelude::*;

impl MemFS {
    #[instrument]
    pub fn new(root_dir: &Path) -> anyhow::Result<Self> {
        let mem_fs = walkdir::WalkDir::new(root_dir)
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        let mem_fs = mem_fs
            .into_par_iter()
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                let file_name = e.file_name().to_str();
                let path = e.path().to_str();
                file_name.map_or(false, |name| {
                    !name.starts_with(".") && !name.ends_with(".excalidraw.md")
                }) && path.map_or(false, |p| {
                    !p.split('/').any(|component| component.starts_with("."))
                })
            })
            .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
            .map(|entry| {
                let path = entry.path();
                let contents = std::fs::read_to_string(path)?;
                let rope = Rope::from_str(&contents);
                let last_modified = path
                    .metadata()
                    .context("Getting metadata")?
                    .modified()
                    .context("Getting system time")?;
                anyhow::Ok((Arc::from(path), (rope, last_modified)))
            })
            .flatten()
            .collect::<HashMap<_, _>>();
        Ok(Self {
            mem_fs: RwLock::new(Arc::new(mem_fs)),
        })
    }

    pub async fn read(&self) -> anyhow::Result<Snapshot> {
        let guard = self.mem_fs.read().await;
        let snapshot = Snapshot(guard.clone());
        Ok(snapshot)
    }
}
