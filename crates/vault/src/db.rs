use std::{
    borrow::{Borrow, Cow},
    collections::{HashMap, HashSet},
    convert::TryInto,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
    u128,
};

use anyhow::Context;
use futures::Future;
use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use redb::{Database, ReadableTable, TableDefinition, TypeName, Value};
use serde::{Deserialize, Serialize};
use tracing::error;
use walkdir::WalkDir;

/// File-synchronized database for arbitrary collections derived from file contents
pub struct FileDB<T: redb::Value> {
    dir: &'static Path,
    _t: std::marker::PhantomData<T>,
}

/// String Relative Path: path of a file relative to the root_dir of the collection of files.
type FileKey = String;
/// Semantic File State: semantic state of the file, indicated by the time that it was last modified
///
/// Maybe in the future this will be a hash or some other state indication.
#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
struct FileState(u128);

/// What is this? This is a monadic interface for constructing (and then running) sync effects for the file-synchonized database.
///
/// What does it do? This allows you to construct the update logic for the sub-sets of your collection that are out of sync,
/// and do so without worrying about the storage mechanisms for these collections or about collection contents that needs to be removed
/// from the database
///
/// Treat and sync the data like a flat collection, even if it chunked up by file for storage.
///
/// When the Item type is the same as the DB storage type, there is the run function. Know that this type has to be a simple type so that it
/// can be stored in the db with close to zero copy. No structs!
pub struct MSync<Item, DBValue: redb::Value> {
    db: FileDB<DBValue>,
    updates: Vec<(FileKey, FileState, Item)>,
    deletes: Vec<FileKey>,
}

use util::ResultIteratorExt;

type FileContent<'a> = &'a str;

impl<V: redb::Value> MSync<(), V> {
    pub async fn async_populate<I: std::fmt::Debug, F: Future<Output = I>>(
        self,
        f: impl Fn(&FileKey, FileContent) -> F + Copy,
    ) -> MSync<I, V> {
        let dir = self.db.dir;
        let futures = self.updates.into_iter().map(|(it, state, _)| async move {
            let key: FileKey = it;
            let root_dir = dir;
            let path = root_dir.join(&key);
            let content = tokio::fs::read_to_string(&path).await?;
            let item = f(&key, &content).await;

            anyhow::Ok((key, state, item))
        });

        let updates = futures::future::join_all(futures)
            .await
            .into_iter()
            .flatten_results_and_log()
            .collect::<Vec<_>>();

        MSync {
            db: self.db,
            deletes: self.deletes,
            updates,
        }
    }
}

impl<I, V: redb::Value> MSync<I, V> {
    pub fn map<IP>(self, f: impl Fn(I) -> IP) -> MSync<IP, V> {
        MSync {
            db: self.db,
            deletes: self.deletes,
            updates: self
                .updates
                .into_iter()
                .map(|(key, state, item)| (key.clone(), state, f(item)))
                .collect(),
        }
    }
}

impl<I, V: redb::Value> MSync<I, V> {
    pub fn flat_map<IP>(self, f: impl Fn(I) -> Vec<IP>) -> MSync<IP, V> {
        MSync {
            db: self.db,
            deletes: self.deletes,
            updates: self
                .updates
                .into_iter()
                .flat_map(|(key, state, item)| {
                    f(item).into_iter().map(move |it| (key.clone(), state, it))
                })
                .collect(),
        }
    }
}

impl<I, V: redb::Value> MSync<I, V> {
    /// External **one-to-one** mapping
    pub async fn external_async_map<IP, F: Future<Output = Vec<IP>>>(
        self,
        f: impl Fn(Vec<I>) -> F,
    ) -> MSync<IP, V> {
        let keys = self
            .updates
            .iter()
            .map(|it| (it.0.to_string(), it.1))
            .collect::<Vec<_>>();
        let old_values = self.updates.into_iter().map(|it| it.2).collect::<Vec<_>>();
        let result = f(old_values).await;

        let updates = keys
            .into_iter()
            .zip(result)
            .map(|it| (it.0 .0, it.0 .1, it.1))
            .collect::<Vec<_>>();

        MSync {
            db: self.db,
            deletes: self.deletes,
            updates,
        }
    }
}

impl<I: 'static> MSync<I, I>
where
    I: for<'a> redb::Value<SelfType<'a> = I>
{
    pub async fn run(self) -> anyhow::Result<FileDB<I>> {
        let MSync {
            db,
            updates,
            deletes,
        } = self;

        let updates: Vec<(FileKey, FileState, Vec<I>)> = updates
            .into_iter()
            .into_group_map_by(|it| (it.0.clone(), it.1))
            .into_iter()
            .map(|it| (it.0, it.1.into_iter().map(|it| it.2).collect()))
            .map(|it| (it.0.0, it.0.1, it.1))
            .collect();



        db.apply_sync(updates, deletes)
    }
}

// yes this is a wacky trait bound but seems to be necessary for redb given our configuration.
impl<T: 'static> FileDB<T>
where
    T: for<'a> redb::Value<SelfType<'a> = T>
{
    const TABLE: TableDefinition<'static, String, (FileState, Vec<T>)> =
        TableDefinition::new("main-table");


    pub fn new(dir: &'static Path) -> Self {
        Self {
            dir,
            _t: std::marker::PhantomData,
        }
    }
       

    pub async fn new_msync(self) -> anyhow::Result<MSync<(), T>> {
        // recursively walk the file directory
        let new_files_state: HashSet<(FileKey, FileState)> = {
            let walker = WalkDir::new(self.dir);

            walker
                .into_iter()
                .flat_map(|it| it)
                .filter(|it| {
                    it.file_type().is_file()
                        && it.path().extension().is_some_and(|extension| {
                            extension.eq_ignore_ascii_case("md")
                                || extension.eq_ignore_ascii_case("markdown")
                        })
                })
                .logging_flat_map(|it| {
                    // ignore any metadata errors; I don't forsee these being an issue
                    let path = it.path();
                    let metadata = path.metadata()?;
                    let file_state: FileState = metadata.modified()?.into();
                    let relative_path = path.strip_prefix(self.dir)?;
                    anyhow::Ok((relative_path.to_string_lossy().into_owned(), file_state))
                })
                .collect()
        };

        let old_files_state: HashSet<(FileKey, FileState)> = self.state()?.unwrap_or_default();

        let diff_new_and_different = new_files_state.difference(&old_files_state);

        let new_paths: HashSet<&FileKey> = new_files_state.iter().map(|(key, _)| key).collect();
        let old_paths: HashSet<&FileKey> = old_files_state.iter().map(|(key, _)| key).collect();

        let removed_paths: HashSet<FileKey> = old_paths
            .difference(&new_paths)
            .into_iter()
            .map(|it| it.to_string())
            .collect();

        Ok(MSync {
            db: self,
            deletes: removed_paths.into_iter().collect(),
            updates: diff_new_and_different
                .map(|(key, state)| (key.to_string(), *state, ()))
                .collect(),
        })
    }


    fn apply_sync(
        self,
        updates: Vec<(FileKey, FileState, Vec<T>)>,
        deletes: Vec<FileKey>,
    ) -> anyhow::Result<Self> {
        let db = Database::create("oxide.db")?;

        let write_txn = db.begin_write()?;

        {
            let mut table = write_txn.open_table(Self::TABLE)?;
            for (key, state, value) in updates {
                table.insert(key, (state, value))?;
            }
            for key in deletes {
                table.remove(key)?;
            }
        }

        write_txn.commit()?;

        Ok(self)
    }

    fn state(&self) -> anyhow::Result<Option<HashSet<(FileKey, FileState)>>> {
        let db = match Database::open("oxide.db") {
            Ok(db) => db,
            Err(redb::DatabaseError::Storage(redb::StorageError::Io(io_error)))
                if io_error.kind() == std::io::ErrorKind::NotFound =>
            {
                // Database file doesn't exist yet, return None
                return Ok(None);
            }
            Err(e) => return Err(e.into()), // Other errors are propagated
        };

        let read_txn = db
            .begin_read()
            .context("Failed to begin read transaction")?;
        let table = read_txn
            .open_table(Self::TABLE)
            .context("Failed to open table")?;

        let result = table
            .iter()?
            .flatten_results_and_log()
            .map(|(key_guard, value_guard)| {
                let key = key_guard.value();
                let (state, _) = value_guard.value();

                (key, state)
            })
            .collect();

        Ok(Some(result))
    }

    pub fn fold<B, F>(&self, init: B, mut f: F) -> anyhow::Result<B>
    where
        F: FnMut(B, &str, &T) -> B,
    {
        let db = Database::open("oxide.db")?;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(Self::TABLE)?;

        table.iter()?
            .flatten_results_and_log()
            .try_fold(init, |acc, (key_guard, value_guard)| {
                let key = key_guard.value();
                let (_, values) = value_guard.value();
                values.iter().try_fold(acc, |inner_acc, value| {
                    let value: &T = value;
                    Ok(f(inner_acc, &key, value))
                })
            })
    }

    pub fn map<U, F: Copy>(&self, f: F) -> anyhow::Result<Vec<U>>
    where
        F: FnOnce(&str, &T) -> U,
    {
        let db = Database::open("oxide.db")?;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(Self::TABLE)?;

        table.iter()?
            .flatten_results_and_log()
            .flat_map(|(key_guard, value_guard)| {
                let key = key_guard.value();
                let (_, values) = value_guard.value();
                values.iter().map(|value| Ok(f(key.as_str(), value))).collect::<Vec<_>>()
            })
            .collect()
    }
}

impl From<SystemTime> for FileState {
    fn from(value: SystemTime) -> Self {
        let t = value
            .duration_since(UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!")
            .as_millis() as u128; // This will truncate if the value is too lar

        FileState(t)
    }
}

impl redb::Value for FileState {
    type SelfType<'a> = Self where Self: 'a;

    type AsBytes<'a> = [u8; std::mem::size_of::<u128>()] where Self: 'a;

    fn fixed_width() -> Option<usize> {
        Some(std::mem::size_of::<FileState>())
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let t = u128::from_le_bytes(data.try_into().expect("Deserializing: Invalid data length")); // this should not happen -- in theory.
        Self(t)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        value.0.to_le_bytes()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("vault::FileState")
    }
}
