
/*
Design. So several things need to happen:
1. In order to work with the files, we must parse files and generate an in-memory representation of them. 
2. In order to get good information on these files (like references for example), we must do analysis on the in-memory representation of the files with specific results in mind; These results are the results align with the LSP types
3. We need to communicate with editors through the LSP types and json rpc

To me, this corresponds to three modules
1. Parsing: This will go through the file system and create an in-memory representation of the files through structs. MDFile, MDHeading, MDTag, MDParagraph with all relevant information about the files. This may just be one public function that returns a main struct representing these things. 
2. Analysis: This will have functions that correspond to relevent LSP Capabilities and will perform analysis on the in-memory representation of the code in order to execute on these capabilities. How will it be used? I think a `new` function will do; this new function could call the parsing function. So this means that the user of the lib (`main.rs`) would not even need to use the parsing module? Ok, so parsing should be a module under the same as analysis.
3. LSP server: This will be in charge of listening to and sending JSON RPC requests. IDK how yet. Ill probably just copy rust-analyzers design
*/

use crate::ide::analysis::Analysis;

// use std::path::PathBuf;
// 
// use analyzer::{analyzer::Analyzer};
// use itertools::Itertools;
// 
// use analyzer::nodes::*;
// use analyzer::graph::Node;
// 
// mod analyzer;
// mod lsp;
mod ide;

fn main() {

    let a = Analysis::new("/home/felix/Notes").unwrap();

    // Get incoming for "Obsidian Text Link Suggestions"

    println!("References: File: Obsidian Text Link Suggestions {:#?}", a.file_incoming("Obsidian Text Link Suggestions"));
    println!("References: File: Practice Reflections.md {:#?}", a.file_incoming("Practice Reflections"));
    println!("References: Heading: cons in Practice Reflections {:#?}", a.heading_incoming("Practice Reflections", "cons"));
    println!("References: Tag: #MapOfContent/apworld: {:#?}", a.tags_incoming("MapOfContent/aplit"));

    // let analyzer = Analyzer::new("/home/felix/Notes");
    // let graph = analyzer.construct_graph();

    // // test 
    // let current_file_path: PathBuf = PathBuf::from("/home/felix/Notes/Obsidian Markdown Language Server.md");
    // let md_file: &MDFile = analyzer.files.get(&current_file_path).unwrap();
    // let incoming_links = md_file.incoming(&graph).unwrap();
    // let names = incoming_links.iter().map(|f| f.name()).collect_vec();
    // println!("Incoming {:#?}", names);
    // let outgoing_links = md_file.outgoing(&graph).unwrap();
    // let names = outgoing_links.iter().map(|f| f.name()).collect_vec();
    // println!("Outgoing {:#?}", names);

    // // Test heading

    // let heading = md_file.headings.iter().find(|h| h.ref_name == "Obsidian Markdown Language Server#Development").unwrap();
    // let heading_incoming = heading.incoming(&graph).unwrap();
    // let names = heading_incoming.iter().map(|f| f.name()).collect_vec();
    // println!("Heading Incoming {:#?}", names);
}


