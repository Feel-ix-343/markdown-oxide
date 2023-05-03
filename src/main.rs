use std::path::PathBuf;

use analyzer::{analyzer::Analyzer, nodes::MDFile, graph::Node};
use itertools::Itertools;

mod analyzer;
mod lsp;

fn main() {
    let analyzer = Analyzer::new("/home/felix/Notes");
    let graph = analyzer.construct_graph();

    // test 
    let current_file_path: PathBuf = PathBuf::from("/home/felix/Notes/Obsidian Markdown Language Server.md");
    let md_file: &MDFile = analyzer.files.get(&current_file_path).unwrap();
    let incoming_links = md_file.incoming(&graph);
    let names = incoming_links.iter().map(|f| f.name()).collect_vec();
    println!("Incoming {:#?}", names);
    let outgoing_links = md_file.outgoing(&graph);
    let names = outgoing_links.iter().map(|f| f.name()).collect_vec();
    println!("Outgoing {:#?}", names);
}


