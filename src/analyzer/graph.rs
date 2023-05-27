use std::{path::PathBuf, collections::HashMap};

use itertools::Itertools;

use super::nodes::{MDFile, MDHeading};

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
    mdfile_outgoing_map: HashMap<PathBuf, Vec<&'a MDFile>>,
    heading_outgoing_map: HashMap<String, Vec<&'a MDFile>>,
    heading_incoming_map: HashMap<String, Vec<&'a MDFile>>
}

impl<'a> Graph<'a> {
    pub fn new(files: &HashMap<PathBuf, MDFile>) -> Graph {
        let incoming_map: HashMap<PathBuf, Vec<&MDFile>> = files.values()
            .flat_map(|f| {
                f.resolved_links.iter().map(|path| (path.to_owned(), f)).collect_vec()
            })
            .into_group_map();

        // let display = incoming_map.iter().map(|(k, v)| format!("File: {:?}, incoming {:#?}", k, v.iter().collect_vec())).join("\n");

        // println!("{display}");

        let outgoing_map: HashMap<PathBuf, Vec<&MDFile>> = files.iter()
            .map(|(p, f)| (p.to_owned(), f.resolved_links.iter().map(|p| files.get(p)).flatten().collect_vec()))
            .collect();

        // need the resolved links that are specifically outgoing links to headings in other files
        let heading_incoming_map: HashMap<String, Vec<&MDFile>> = files.iter()
            .flat_map(|(p, f)| {
                f.resolved_heading_links.iter().map(|(path, heading_ref)| (heading_ref.to_owned(), f)).collect_vec()
            })
            .into_group_map();


        let heading_outgoing_map: HashMap<String, Vec<&MDFile>> = files.iter()
            .flat_map(|(p, f)| {
                f.headings.iter().map(|h| {
                    let resolved_links = h.resolved_links.iter().map(|p| files.get(p)).flatten().collect_vec();
                    (h.ref_name.to_owned(), resolved_links)
                })
            })
            .collect();

        return Graph {
            mdfile_incoming_map: incoming_map,
            mdfile_outgoing_map: outgoing_map,
            heading_incoming_map,
            heading_outgoing_map
        }
    }

    fn incoming(&self, file: &MDFile) -> Option<&Vec<&MDFile>> {
        let incoming = self.mdfile_incoming_map.get(&file.path)?;
        return Some(incoming)
    }

    fn outgoing(&self, file: &MDFile) -> Option<&Vec<&MDFile>> { 
        let outgoing = self.mdfile_outgoing_map.get(&file.path)?;
        return Some(outgoing)
    }
}



// for a file: parse for nodes -> calculate incoming and outgoing for each node
// ex: parse for headings and in-file-references -> calculate which refs are to headings (assuming that no headings have references)

// list of all nodes -> iter through each -> attach the outgoing links and link to the outgoing link's incoming link

pub trait Node {
    fn incoming<'a>(&'a self, ctx: &'a Graph) -> Option<Vec<&'a dyn Node>>;
    fn outgoing<'a>(&'a self, ctx: &'a Graph) -> Option<Vec<&'a dyn Node>>;
    fn name(&self) -> &str;
}

impl Node for MDFile {
    fn outgoing<'a>(&'a self, ctx: &'a super::graph::Graph) -> Option<Vec<&'a dyn Node>> {
        let outgoing_files = ctx.outgoing(&self)?;
        let outgoing_nodes: Vec<&dyn Node> = outgoing_files.into_iter()
            .map(|&f| f as &dyn Node)
            .collect_vec();
        return Some(outgoing_nodes)
    }

    fn incoming<'a>(&'a self, ctx: &'a super::graph::Graph) -> Option<Vec<&'a dyn Node>> {
        let incoming_files = ctx.incoming(&self)?;
        let incoming_nodes = incoming_files.into_iter()
            .map(|&f| f as &dyn Node)
            .collect_vec();
        return Some(incoming_nodes)
    }

     fn name(&self) -> &str {
         return self.title()
     }
}

impl Node for MDHeading {
    fn name(&self) -> &str {
        &self.heading
    }
    fn incoming<'a>(&'a self, ctx: &'a Graph) -> Option<Vec<&'a dyn Node>> {
        let incoming_files = ctx.heading_incoming_map.get(&self.ref_name)?;  
        let incoming_nodes = incoming_files.into_iter()
            .map(|&f| f as &dyn Node)
            .collect_vec();

        // figure out incoming headings?

        return Some(incoming_nodes)
    }
    fn outgoing<'a>(&'a self, ctx: &'a Graph) -> Option<Vec<&'a dyn Node>> {
        let outgoing_files = ctx.heading_outgoing_map.get(&self.ref_name)?;
        let outgoing_nodes = outgoing_files.into_iter()
            .map(|&f| f as &dyn Node)
            .collect_vec();

        return Some(outgoing_nodes)
    }
}
