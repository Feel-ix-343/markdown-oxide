use std::path::PathBuf;

use itertools::Itertools;

use super::{markdownparser::MarkdownLinkParser, graph::Node};


#[derive(Debug)]
pub struct MDFile {
    pub path: PathBuf,
    source: Vec<u8>,
    home_dir: PathBuf,

    // pub title: &'a str, // Could these be functions so that they don't need to be cloned?
    // pub links: Vec<&'a str>, // Could these be functions so that they don't need to be cloned?
}
pub struct MDHeading;
pub struct MDTag;

impl<'a> MDFile {
    pub fn new(path: PathBuf, home_dir: PathBuf) -> MDFile {
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

impl Node for MDFile {
    fn outgoing<'a>(&'a self, ctx: &'a super::graph::Graph) -> Vec<&'a dyn Node> {
        let outgoing_files = ctx.outgoing(&self).unwrap();
        let outgoing_nodes: Vec<&dyn Node> = outgoing_files.into_iter()
            .map(|&f| f as &dyn Node)
            .collect_vec();
        return outgoing_nodes
    }

    fn incoming<'a>(&'a self, ctx: &'a super::graph::Graph) -> Vec<&'a dyn Node> {
        let incoming_files = ctx.incoming(&self).unwrap();
        let incoming_nodes = incoming_files.into_iter()
            .map(|&f| f as &dyn Node)
            .collect_vec();
        return incoming_nodes
    }
}
