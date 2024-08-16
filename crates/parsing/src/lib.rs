// struct QueryBlock {
//     sub_blocks: Option<Vec<Arc<QueryBlock>>>,
//     parent: Option<Arc<QueryBlock>>,
//
//     /// Rendered TExt
//     queryable_text: String,
//     collections: Vec<Arc<Collection>>,
// }

pub use documents::Documents;

pub mod document;

mod documents {
    use std::{collections::HashMap, ops::Deref, path::Path, sync::Arc, time::Duration};

    use itertools::Itertools;
    use walkdir::WalkDir;

    use crate::document::Document;

    use rayon::prelude::*;

    pub struct Documents {
        documents: HashMap<Arc<Path>, Document>,
        root_dir: Arc<Path>,
    }

    impl Deref for Documents {
        type Target = HashMap<Arc<Path>, Document>;

        fn deref(&self) -> &Self::Target {
            &self.documents
        }
    }

    impl Documents {
        pub fn documents(&self) -> &HashMap<Arc<Path>, Document> {
            &self.documents
        }
    }

    impl Documents {
        pub fn from_root_dir(root_dir: &Path) -> Self {
            let md_file_paths = WalkDir::new(root_dir)
                .into_iter()
                .filter_entry(|e| {
                    !e.file_name()
                        .to_str()
                        .map(|s| s.starts_with('.') || s == "logseq") // TODO: This is a temporary fix; a hidden config is better
                        .unwrap_or(false)
                })
                .flatten()
                .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
                .collect_vec();

            let now = std::time::Instant::now();
            let documents: HashMap<Arc<Path>, Document> = md_file_paths
                .par_iter()
                .flat_map(|p| {
                    let text = std::fs::read_to_string(p.path()).ok()?;
                    let document = Document::new(&text)?;

                    return Some((p.path().into(), document));
                })
                .collect();

            Documents {
                documents,
                root_dir: Arc::from(root_dir),
            }
        }
    }
}
