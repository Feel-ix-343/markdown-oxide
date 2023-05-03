use std::{path::{PathBuf, Path}, collections::HashMap};

use itertools::Itertools;

use super::{nodes::MDFile, graph::Graph};


#[derive(Debug)]
/// The base struct for making calculations on the vault of markdown files. It saves the states of the current directory and a list of all of the files in the vault
pub struct Analyzer {
    pub files: HashMap<PathBuf, MDFile>,
    directory: PathBuf,
}

impl Analyzer {
    /// Creates a new analyzer struct
    pub fn new(directory: &str) -> Analyzer {
        let directory = Path::new(directory).to_owned();
        assert!(directory.is_dir());

        let files: HashMap<PathBuf, MDFile> = std::fs::read_dir(directory.to_owned())
            .unwrap()
            .map(|f| f.unwrap())
            .collect_vec()
            .into_iter()
            .filter(|f| f.path().is_file() && f.path().to_str().unwrap().ends_with(".md"))
            .map(|p| (p.path().to_owned(), MDFile::new(p.path(), directory.to_owned())))
            .collect();
        
        return Analyzer {
            files,
            directory
        };
    }

    pub fn construct_graph(&self) -> Graph {
        let graph = Graph::new(&self.files, self.directory.to_owned());
        return graph
    }
}
