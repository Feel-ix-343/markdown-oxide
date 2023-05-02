use std::{path::{PathBuf, Path}, fs::DirEntry, collections::HashMap, net::Incoming};

use itertools::Itertools;
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{MarkdownParser, inline_language};

pub mod graph;

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
            .map(|p| MDFile::new(p, directory.to_owned()))
            .collect();
        
        return Analyzer {
            files,
            directory
        };
    }

    pub fn calc_incoming(&self) {

        let incoming_map: HashMap<PathBuf, Vec<&MDFile>> = self.files.iter()
            .flat_map(|f| {
                f.resolved_links().into_iter().map(|path| (path, f)).collect_vec()
            })
            .into_group_map();

        let display = incoming_map.iter().map(|(k, v)| format!("File: {:?}, incoming {:#?}", k, v.iter().map(|f| f.title()).collect_vec())).join("\n");

        println!("{display}");
    }
}

#[derive(Debug)]
pub struct MDFile {
    pub path: PathBuf,
    source: Vec<u8>,
    home_dir: PathBuf
    // pub title: &'a str, // Could these be functions so that they don't need to be cloned?
    // pub links: Vec<&'a str>, // Could these be functions so that they don't need to be cloned?
}
pub struct MDHeading;
pub struct MDTag;

impl<'a> MDFile {
    pub fn new(dir_entry: DirEntry, home_dir: PathBuf) -> MDFile {
        let path = dir_entry.path();
        let source = std::fs::read(&path).unwrap();

        MDFile {
            path,
            source,
            home_dir
        }
    }

    pub fn title(&self) -> &str {
        return self.path.file_name().unwrap().to_str().unwrap()
    }

    /// All of the resolved, relative links in a markdown file
    pub fn resolved_links(&self) -> Vec<PathBuf> {
        let mut parser = MarkdownLinkParser::new();
        let links = parser.links_for_file(&self.source); 

        let paths = links.into_iter()
            .map(|s| {
                let mut path = PathBuf::new();
                path.push(&self.home_dir); // 
                path.push(s);
                let path = path.with_extension("md");
                path
            }) // Turn into full path
            .filter(|p| p.is_file())
            .collect_vec();

        return paths
    }
}


// constructor impls
// node impls
// node outgoing parsers (ex: file node parses for links in the file; each of these links are outgoing edges)
// TODO: pub struct MDWordSequence


struct MarkdownLinkParser {
    parser: MarkdownParser,
}

impl MarkdownLinkParser {
    pub fn new() -> MarkdownLinkParser {
        MarkdownLinkParser {
            parser: MarkdownParser::default()
        }
    }

    fn links_for_file<'a>(&mut self, source_code: &'a [u8]) -> Vec<&'a str> {
        let parser = &mut self.parser;

        let tree = parser.parse(source_code, None).unwrap();
        // let language = language();
        let inline_language = inline_language();

        // let block_tree = tree.block_tree();
        let inline_trees = tree.inline_trees();

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
