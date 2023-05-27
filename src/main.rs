use std::path::PathBuf;

use analyzer::{analyzer::Analyzer};
use itertools::Itertools;

use analyzer::nodes::*;
use analyzer::graph::Node;

mod analyzer;
mod lsp;

fn main() {
    let analyzer = Analyzer::new("/home/felix/Notes");
    let graph = analyzer.construct_graph();

    // test 
    let current_file_path: PathBuf = PathBuf::from("/home/felix/Notes/Obsidian Markdown Language Server.md");
    let md_file: &MDFile = analyzer.files.get(&current_file_path).unwrap();
    let incoming_links = md_file.incoming(&graph).unwrap();
    let names = incoming_links.iter().map(|f| f.name()).collect_vec();
    println!("Incoming {:#?}", names);
    let outgoing_links = md_file.outgoing(&graph).unwrap();
    let names = outgoing_links.iter().map(|f| f.name()).collect_vec();
    println!("Outgoing {:#?}", names);

    // Test heading

    let heading = md_file.headings.iter().find(|h| h.ref_name == "Obsidian Markdown Language Server#Development").unwrap();
    let heading_incoming = heading.incoming(&graph).unwrap();
    let names = heading_incoming.iter().map(|f| f.name()).collect_vec();
    println!("Heading Incoming {:#?}", names);
}


