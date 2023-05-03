use std::{path::PathBuf, collections::HashMap};

use itertools::Itertools;
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{MarkdownParser, inline_language};

use super::MDFile;

// NODE: file, headings, tags, ... ... maybe highlighted text, prob different though

// NOTES: Functionality
// 1. Get incoming references for all node types
// 2. Get outgoing references from current file

// Potential Applications (in no order)
// - Renaming node renames in all incoming
// - Go to definition for nodes
// - References for a node
// - Transform to node suggestions

pub struct Graph {
    incoming_map: HashMap<PathBuf, Vec<PathBuf>>,
    outgoing_map: HashMap<PathBuf, Vec<PathBuf>>
}

impl Graph {
    pub fn new(files: &Vec<MDFile>) -> Graph {
        let incoming_map: HashMap<PathBuf, Vec<PathBuf>> = files.iter()
            .flat_map(|f| {
                f.resolved_links().into_iter().map(|path| (path, f.path.to_owned())).collect_vec()
            })
            .into_group_map();

        // let display = incoming_map.iter().map(|(k, v)| format!("File: {:?}, incoming {:#?}", k, v.iter().collect_vec())).join("\n");

        // println!("{display}");

        let outgoing_map: HashMap<PathBuf, Vec<PathBuf>> = files.iter().map(|f| (f.path.to_owned(), f.resolved_links())).collect();

        return Graph {
            incoming_map,
            outgoing_map
        }
    }

    pub fn incoming(&self, file: &MDFile) -> Option<&Vec<PathBuf>> {
        let incoming = self.incoming_map.get(&file.path)?;
        return Some(incoming)
    }

    pub fn outgoing(&self, file: &MDFile) -> Option<&Vec<PathBuf>> { 
        let outgoing = self.outgoing_map.get(&file.path)?;
        return Some(outgoing)
    }
}



// for a file: parse for nodes -> calculate incoming and outgoing for each node
// ex: parse for headings and in-file-references -> calculate which refs are to headings (assuming that no headings have references)

// list of all nodes -> iter through each -> attach the outgoing links and link to the outgoing link's incoming link
