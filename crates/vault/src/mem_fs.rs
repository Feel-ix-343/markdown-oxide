use std::{
    borrow::Cow,
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use anyhow::{anyhow, Context};
use derive_deref::Deref;
use itertools::Itertools;
use md_parser::Document;
use rayon::iter::{IntoParallelIterator, ParallelBridge};
use ropey::Rope;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, trace};

/// Metadata, File (giving access to the text of the file)
type MemFSValue = (Metadata, FsFile);
/// Relative Path -> (Metadata, File)
type MemFSMap = HashMap<String, MemFSValue>;

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
                let metadata = Metadata { last_modified };
                let file = FsFile(rope);
                let relative_path = path
                    .strip_prefix(root_dir)
                    .context("Getting relative path")?
                    .to_str()
                    .context("Converting path to string")?
                    .to_string();
                anyhow::Ok((relative_path, (metadata, file)))
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

/// Map of Relative Paths for markdown files to file data (string, eg folder/file is root_dir/folder/file.md)
///
/// RelativePath (Id) -> (Metadata, File)
/// pretty cheap to clone for now too
#[derive(Debug, Clone)]
pub struct Snapshot(Arc<MemFSMap>);
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub last_modified: SystemTime,
}
#[derive(Debug)]
pub struct FsFile(Rope);

impl Snapshot {
    /// Get by relative name of the file, the id
    ///
    /// this is the same type as the index key, so this should be pretty easy to call.
    pub fn get<'a>(&'a self, id: &str) -> anyhow::Result<&MemFSValue> {
        self.0.get(id).ok_or(anyhow!("Relative Path doesn't exist"))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &MemFSValue)> {
        self.0.iter()
    }
}

impl FsFile {
    pub fn get_line(&self, line_index: usize) -> anyhow::Result<Cow<str>> {
        self.0
            .get_line(line_index)
            .map(|line| line.into())
            .ok_or_else(|| anyhow!("Line index out of bounds"))
    }

    /// Get lines inclusive
    pub fn get_lines(&self, range: std::ops::Range<usize>) -> anyhow::Result<Cow<str>> {
        let start = range.start;
        let end = range.end;
        if start > end {
            return Err(anyhow!("Invalid range: start > end"));
        }
        let slice = self
            .0
            .get_slice(self.0.line_to_char(start)..self.0.line_to_char(end + 1))
            .ok_or_else(|| anyhow!("Failed to get slice"))?;
        Ok(slice.into())
    }

    pub fn text(&self) -> Cow<str> {
        (&self.0).into()
    }
}
