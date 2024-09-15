use std::{collections::HashMap, path::Path, sync::Arc};

use util::Collection;

pub struct DB {
    documents: Documents,
    index: index::Index,
}

pub struct DBConfig {
    db_path: &'static Path,
    vault_path: &'static Path,
}

mod index;

mod util {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Collection<T> {
        pub first: T,
        pub rest: Vec<T>,
    }

    impl<T> Collection<T> {
        pub fn iter(&self) -> impl Iterator<Item = &T> {
            std::iter::once(&self.first).chain(self.rest.iter())
        }
    }
}
