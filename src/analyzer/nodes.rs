use std::{path::PathBuf};

use itertools::Itertools;

use super::markdownparser::MarkdownNodeParser;

#[derive(Debug)]
pub struct MDFile {
    pub path: PathBuf,
    source: Vec<u8>,
    home_dir: PathBuf,
    pub resolved_links: Vec<PathBuf>,
    pub resolved_heading_links: Vec<(PathBuf, String)>,
    pub headings: Vec<MDHeading>,

    // pub title: &'a str, // Could these be functions so that they don't need to be cloned?
    // pub links: Vec<&'a str>, // Could these be functions so that they don't need to be cloned?
}


impl<'a> MDFile {
    pub fn new(path: PathBuf, home_dir: PathBuf) -> MDFile {
        let source = std::fs::read(&path).unwrap();

        let mut parser = MarkdownNodeParser::new();

        // Resolved links
        let links = parser.links_for_source(&source); 
        let paths = links.iter()
            .map(|s| {
                let mut path = PathBuf::new();
                path.push(&home_dir); // 
                path.push(s);
                let path = path.with_extension("md");
                path
            }) // Turn into full path
            .filter(|p| p.is_file())
            .collect_vec();

        // Resolved heading links

        // The path of the file that the heading is in and the ref of the heading.
        let heading_links: Vec<(PathBuf, String)> = links.into_iter()
            .filter(|&i| i.contains("#") && !i.contains("^"))
            .map(|i| {
                let sections = i.split("#").collect_vec();

                // turn this into a function
                let mut path = PathBuf::new();
                path.push(&home_dir); // 
                path.push( sections[0] );
                let path = path.with_extension("md");

                return (path, i.to_owned())
            })
            .collect_vec();


        let headings = parser.headings_for_file(&source);
        let MDHeadings: Vec<MDHeading> = headings.into_iter()
            .map(|t| t.trim_start())
            .map(|t| MDHeading::new(t, path.clone(), &home_dir))
            .collect_vec();



        MDFile {
            path,
            source,
            resolved_links: paths,
            resolved_heading_links: heading_links,
            headings: MDHeadings,
            home_dir
        }
    }

    pub fn title(&self) -> &str {
        return self.path.file_name().unwrap().to_str().unwrap()
    }
}


#[derive(Debug)]
pub struct MDHeading {
    pub heading: String,
    pub ref_name: String,
    pub resolved_links: Vec<PathBuf>
}

impl MDHeading {
    fn new (heading: &str, file_name: PathBuf, home_dir: &PathBuf) -> MDHeading {

        let ref_name = format!("{}#{}", file_name.file_stem().unwrap().to_str().unwrap(), heading);

        let mut parser = MarkdownNodeParser::new();
        let headingtext = heading.bytes().collect_vec();
        let links = parser.links_for_source(&headingtext);
        let paths = links.iter()
            .map(|l| {
                let mut path = PathBuf::new();
                path.push(&home_dir); // 
                path.push(l);
                let path = path.with_extension("md");
                path
            })
            .collect_vec();

        return MDHeading {
            heading: heading.to_string(),
            ref_name,
            resolved_links: paths
        }
    }
}


// Needs to get the outgoing links


// TODO: pub struct MDHeading;
// TODO: pub struct MDTag;
