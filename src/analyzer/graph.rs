use std::{path::PathBuf, collections::HashMap};

use itertools::Itertools;

pub trait Node {
    fn incoming<'a>(&'a self, ctx: &'a Graph) -> Vec<&'a dyn Node>;
    fn outgoing<'a>(&'a self, ctx: &'a Graph) -> Vec<&'a dyn Node>;
    fn name(&self) -> &str;
}


use super::nodes::MDFile;

// NODE: file, headings, tags, ... ... maybe highlighted text, prob different though

// NOTES: Functionality
// 1. Get incoming references for all node types
// 2. Get outgoing references from current file

// Potential Applications (in no order)
// - Renaming node renames in all incoming
// - Go to definition for nodes
// - References for a node
// - Transform to node suggestions

pub struct Graph<'a> {
    mdfile_incoming_map: HashMap<PathBuf, Vec<&'a MDFile>>,
    mdfile_outgoing_map: HashMap<PathBuf, Vec<&'a MDFile>>
}

impl<'a> Graph<'a> {
    pub fn new(files: &HashMap<PathBuf, MDFile>, home_dir: PathBuf) -> Graph {
        let incoming_map: HashMap<PathBuf, Vec<&MDFile>> = files.values()
            .flat_map(|f| {
                f.resolved_links().into_iter().map(|path| (path, f)).collect_vec()
            })
            .into_group_map();

        // let display = incoming_map.iter().map(|(k, v)| format!("File: {:?}, incoming {:#?}", k, v.iter().collect_vec())).join("\n");

        // println!("{display}");

        let outgoing_map: HashMap<PathBuf, Vec<&MDFile>> = files.iter()
            .map(|(p, f)| (p.to_owned(), f.resolved_links().iter().map(|p| files.get(p).unwrap()).collect_vec()))
            .collect();

        return Graph {
            mdfile_incoming_map: incoming_map,
            mdfile_outgoing_map: outgoing_map
        }
    }

    pub fn incoming(&self, file: &MDFile) -> Option<&Vec<&MDFile>> {
        let incoming = self.mdfile_incoming_map.get(&file.path)?;
        return Some(incoming)
    }

    pub fn outgoing(&self, file: &MDFile) -> Option<&Vec<&MDFile>> { 
        let outgoing = self.mdfile_outgoing_map.get(&file.path)?;
        return Some(outgoing)
    }
}



// for a file: parse for nodes -> calculate incoming and outgoing for each node
// ex: parse for headings and in-file-references -> calculate which refs are to headings (assuming that no headings have references)

// list of all nodes -> iter through each -> attach the outgoing links and link to the outgoing link's incoming link
