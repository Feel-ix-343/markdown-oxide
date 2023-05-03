use std::path::PathBuf;

use itertools::Itertools;

use super::markdownparser::MarkdownLinkParser;


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
