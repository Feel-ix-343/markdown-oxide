use std::{path::PathBuf, fs::DirEntry};

use itertools::Itertools;
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{MarkdownParser, inline_language};

pub mod graph;

#[derive(Debug)]
pub struct Analyzer {
    pub files: Vec<MDFile>
}

impl Analyzer {
    pub fn new(directory: &str) -> Analyzer {

        let files: Vec<MDFile> = std::fs::read_dir(directory)
            .unwrap()
            .map(|f| f.unwrap())
            .collect_vec()
            .into_iter()
            .filter(|f| f.path().is_file() && f.path().to_str().unwrap().ends_with(".md"))
            .map(MDFile::new)
            .collect();

        return Analyzer {
            files
        };

    }
}

#[derive(Debug)]
pub struct MDFile {
    pub path: PathBuf,
    source: Vec<u8>,
    // pub title: &'a str, // Could these be functions so that they don't need to be cloned?
    // pub links: Vec<&'a str>, // Could these be functions so that they don't need to be cloned?
}
pub struct MDHeading;
pub struct MDTag;

impl<'a> MDFile {
    pub fn new(dir_entry: DirEntry) -> MDFile {
        let path = dir_entry.path();
        let source = std::fs::read(&path).unwrap();

        MDFile {
            path,
            source,
        }
    }

    pub fn title(&self) -> &str {
        return self.path.file_name().unwrap().to_str().unwrap()
    }

    pub fn links(&self) -> Vec<&str> {
        let parser = MarkdownLinkParser::new();
        let links = parser.links_for_file(&self.source); 
        return links
    }
}


// constructor impls
// node impls
// node outgoing parsers (ex: file node parses for links in the file; each of these links are outgoing edges)
// TODO: pub struct MDWordSequence


struct MarkdownLinkParser {
    parser: MarkdownParser
}

impl MarkdownLinkParser {
    pub fn new() -> MarkdownLinkParser {
        MarkdownLinkParser {
            parser: MarkdownParser::default()
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
                let captures = query_cursor.captures(&query, tree.root_node(), text_provider).collect_vec();
                return captures.into_iter().flat_map(|(q, _)| q.captures).map(|c| c.node.utf8_text(source_code).unwrap()).collect_vec()
            }) // Map each tree to its query captures, then flatten all trees to a collection of their query captures
            .collect(); // TODO: I still want to refactor this more

        return links
    }
}
