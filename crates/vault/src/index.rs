use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use derive_deref::Deref;
use redb::{Database, ReadableTable, TableDefinition, Value};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

/// methods
impl<V: IndexValue> Index<V> {
    /// Creates, updates, or deletes data stored in file
    pub async fn sync_file(self, key: &str, data: Option<V>) -> anyhow::Result<Index<V>> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(self.table)?;
            let key = Key(Cow::Borrowed(key));
            match data {
                Some(value) => table.insert(&key, &ValueWrapper(value))?,
                None => table.remove(&key)?,
            };
        }
        write_txn.commit()?;
        Ok(self)
    }

    /// Creates, updates, or deletes data stored in files
    pub async fn sync_files(&self, data: Vec<(&str, Option<V>)>) -> anyhow::Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(self.table)?;
            for (key, value) in data {
                let key = Key(key.into());
                match value {
                    Some(v) => table.insert(key, &ValueWrapper(v))?,
                    None => table.remove(key)?,
                };
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn map<F, R>(&self, f: F) -> anyhow::Result<Option<Vec<R>>>
    where
        F: Fn(Key<'_>, V) -> anyhow::Result<R>,
    {
        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(self.table) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_e)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let iter_result = table
            .iter()
            .context("Getting table iterator")?
            .map(|item| {
                let (key_guard, value_guard) = item?;
                let key = key_guard.value();
                let value = value_guard.value().0;

                f(key, value)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Some(iter_result))
    }

    pub fn fold_left<F, R>(&self, init: R, f: F) -> anyhow::Result<R>
    where
        F: Fn(R, Key<'_>, V) -> anyhow::Result<R>,
    {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;

        table
            .iter()
            .context("Getting table iterator")?
            .try_fold(init, |acc, item| {
                let (key_guard, value_guard) = item?;
                let key = key_guard.value();
                let value = value_guard.value().0;

                f(acc, key, value)
            })
    }

    pub async fn get_single(&self, key: Key<'_>) -> anyhow::Result<Option<V>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        let result = table.get(key)?;
        Ok(result.map(|v| v.value().0))
    }
}

impl<V: IndexValue> Index<V> {
    pub async fn new(db_path: &Path) -> anyhow::Result<Self> {
        let db = Database::create(db_path).context("Creating database")?;
        let table = TableDefinition::new("main_table");
        Ok(Index { db, table })
    }
}

/// Disk index
pub(crate) struct Index<V: IndexValue> {
    db: Database,
    table: TableDefinition<'static, Key<'static>, ValueWrapper<V>>, // the lifetime here doesn't matter
}

#[derive(Debug, Deref)]
/// Id: relative path of file
pub struct Key<'a>(Cow<'a, str>);

impl<'a> Key<'a> {
    pub fn from_str(s: &'a str) -> Self {
        Key(Cow::Borrowed(s))
    }
}

impl redb::Key for Key<'_> {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        data1.cmp(data2)
    }
}

impl<'k> redb::Value for Key<'k> {
    type SelfType<'a> = Key<'a> where Self: 'a;
    type AsBytes<'a> = &'a [u8] where Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Key(Cow::Borrowed(std::str::from_utf8(data).unwrap())) // todo this might be an issue
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a + 'b,
    {
        value.0.as_bytes()
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Key")
    }
}

pub trait IndexValue: Debug + 'static {
    fn as_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self
    where
        Self: Sized;
}

use std::fmt::Debug;
#[derive(Debug)]
struct ValueWrapper<V: IndexValue>(V);

impl<V: IndexValue> redb::Value for ValueWrapper<V> {
    type SelfType<'a> = ValueWrapper<V> where Self: 'a;
    type AsBytes<'a> = Vec<u8> where Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let v = V::from_bytes(data);
        ValueWrapper(v)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a + 'b,
    {
        value.0.as_bytes()
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("ValueWrapper")
    }
}
