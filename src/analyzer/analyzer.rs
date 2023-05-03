use std::path::{PathBuf, Path};

use itertools::Itertools;

use super::{nodes::MDFile, graph::Graph};


#[derive(Debug)]
pub struct Analyzer {
    pub files: Vec<MDFile>,
    directory: PathBuf,
}

impl Analyzer {
    pub fn new(directory: &str) -> Analyzer {
        let directory = Path::new(directory).to_owned();
        assert!(directory.is_dir());

        let files: Vec<MDFile> = std::fs::read_dir(directory.to_owned())
            .unwrap()
            .map(|f| f.unwrap())
            .collect_vec()
            .into_iter()
            .filter(|f| f.path().is_file() && f.path().to_str().unwrap().ends_with(".md"))
            .map(|p| MDFile::new(p.path(), directory.to_owned()))
            .collect();
        
        return Analyzer {
            files,
            directory
        };
    }

    pub fn construct_graph(&self) -> Graph {
        let graph = Graph::new(&self.files);
        return graph
    }
}
