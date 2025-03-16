use std::path::PathBuf;

use ::itertools::Itertools;
use ::rayon::prelude::*;
use ::tree_sitter::{Query, TextProvider};
use ::tree_sitter_md::MarkdownParser;
use ::tree_sitter_md::{inline_language, language};
use tree_sitter::QueryCursor;

fn main() {
    // Read the ../TestFiles/Test.md
    // let test_md_file = std::fs::read("./TestFiles/Test.md").unwrap();
    let test_files: Vec<(String, Vec<u8>)> = std::fs::read_dir("/home/felix/Notes")
        .unwrap()
        .map(|f| f.unwrap())
        .collect_vec()
        .par_iter()
        .map(|f| f.path())
        .filter(|f| f.is_file())
        .filter(|f| f.file_name().unwrap().to_str().unwrap().ends_with(".md"))
        .map(|p| (p.to_str().unwrap().to_owned(), std::fs::read(p).unwrap()))
        .collect();

    let links: Vec<(&String, Vec<&str>)> = test_files
        .par_iter()
        .map(|(file, source)| {
            let parser = MarkdownLinkParser::new();
            (file, parser.links_for_file(&source))
        })
        .collect();

    // Pring the matches
    println!("LOOK HERE; the links in the file:\n{:#?}", links)
}

struct MarkdownLinkParser {
    parser: MarkdownParser,
}

impl MarkdownLinkParser {
    fn new() -> MarkdownLinkParser {
        MarkdownLinkParser {
            parser: MarkdownParser::default(),
        }
    }
    fn links_for_file<'a>(self, source_code: &'a [u8]) -> Vec<&'a str> {
        let mut parser = self.parser;

        let tree = parser.parse(source_code, None).unwrap();
        // let language = language();
        let inline_language = inline_language();

        // let block_tree = tree.block_tree();
        let inline_trees = tree.inline_trees();

        // println!("{:?}", block_tree.root_node().to_sexp());
        // inline_trees.iter().for_each(|node| println!("{:?}", node.root_node().to_sexp()));

        // Finding links in the files

        // Execute a treesitter query for finding the links from a file
        let query = Query::new(inline_language, "(link_text) @link;").unwrap();

        let mut query_cursor = QueryCursor::new();
        let text_provider: &[u8] = &[];

        let links: Vec<&str> = inline_trees // There are multiple inline trees
            .iter() // Iterate over each of them
            .flat_map(|tree| {
                let captures = query_cursor
                    .captures(&query, tree.root_node(), text_provider)
                    .collect_vec();
                return captures
                    .into_iter()
                    .flat_map(|(q, _)| q.captures)
                    .map(|c| c.node.utf8_text(source_code).unwrap())
                    .collect_vec();
            }) // Map each tree to its query captures, then flatten all trees to a collection of their query captures
            .collect(); // TODO: I still want to refactor this more

        return links;
    }
}
