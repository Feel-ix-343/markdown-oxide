use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{parser::QueryMetadata, querier::QuerierCache};

#[derive(Debug, Default)]
pub struct QueryCache {
    pub(crate) previous_metadata: Option<QueryMetadata>,
    pub(crate) querier_cache: QuerierCache,
}

impl QueryCache {
    pub(crate) fn clear(&mut self) {
        self.previous_metadata = None;
        self.querier_cache.clear();
    }
}
