// struct QueryBlock {
//     sub_blocks: Option<Vec<Arc<QueryBlock>>>,
//     parent: Option<Arc<QueryBlock>>,
//
//     /// Rendered TExt
//     queryable_text: String,
//     collections: Vec<Arc<Collection>>,
// }

mod blocks;
mod document;

mod documents {
    use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

    use itertools::Itertools;
    use walkdir::WalkDir;

    use crate::document::Document;

    use rayon::prelude::*;

    pub(crate) struct Documents {
        documents: HashMap<Arc<Path>, Document>,
        root_dir: Arc<Path>,
    }

    impl Documents {
        pub(crate) fn documents(&self) -> &HashMap<Arc<Path>, Document> {
            &self.documents
        }
    }

    impl Documents {
        fn from_root_dir(root_dir: &Path) -> Self {
            let now = std::time::Instant::now();
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
            println!("WalkDir: {:?}", now.elapsed());

            let now = std::time::Instant::now();
            let documents: HashMap<Arc<Path>, Document> = md_file_paths
                .par_iter()
                .flat_map(|p| {
                    let text = std::fs::read_to_string(p.path()).ok()?;
                    let document = Document::new(&text)?;

                    return Some((p.path().into(), document));
                })
                .collect();
            println!("Read files: {:?}", now.elapsed());

            Documents {
                documents,
                root_dir: Arc::from(root_dir),
            }
        }
    }

    // tests
    #[cfg(test)]
    mod tests {
        use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};

        use anyhow::Context;
        use rayon::prelude::*;

        use crate::blocks::{BlockCx, Blocks};

        use super::Documents;

        #[test]
        fn bench() -> anyhow::Result<()> {
            let now = std::time::Instant::now();
            let path = PathBuf::from_str("/home/felix/notes")?;
            let documents = Documents::from_root_dir(&path);

            let partial_block_cx = BlockCx::new(&documents, &path);
            let blocks: HashMap<_, _> = documents
                .documents
                .par_iter()
                .map(|(p, document)| {
                    let block_cx = partial_block_cx(p);
                    (
                        p,
                        Blocks::new(block_cx, document)
                            .context(format!("Constructing blocks for path: {p:?}"))
                            .unwrap(),
                    )
                })
                .collect();

            println!("Blocks: {:?}", now.elapsed());

            // print blocks in 2024-08-05
            println!(
                "Blocks: {:#?}",
                documents
                    .documents
                    .get(&Arc::from(path.join("2024-08-05.md")))
            );

            // println!(
            //     "Blocks: {:#?}",
            //     blocks.get(&Arc::from(path.join("2024-08-05.md")))
            // );

            let elapsed = now.elapsed();
            println!("Elapsed: {:?}", elapsed);
            assert!(elapsed.as_secs() < 1);

            Ok(())
        }
    }
}
