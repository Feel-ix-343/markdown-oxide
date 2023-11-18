use std::{path::{PathBuf, Path}, collections::HashMap, fs::{DirEntry, read}, ffi::OsString, ops::Range};

use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;

pub (super) struct Vault {
    pub (super) files: HashMap<String, MDFile>
}

impl Vault {
    pub (super) fn new(vault_dir: &str) -> Result<Vault, std::io::Error> {
        let dir_path = Path::new(vault_dir).to_owned();



        let files = dir_path
            .read_dir()?
            .filter_map(|f| Result::ok(f))
            .collect_vec();

        let md_files = files.iter()
            .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
            .map(|f| {

                let md_file = MDFile::new(f.path().to_str().unwrap()); 

                let relative_path = f.path().strip_prefix(&dir_path).unwrap().to_owned();
                let ref_name = relative_path.file_stem().unwrap().to_owned();

                return (ref_name.to_str().unwrap().to_owned(), md_file)
            })
            .collect();


        return Ok(Vault { files: md_files })

    }

    pub(super) fn get_linking_nodes(&self) -> Vec<&dyn Linking> {
        return self.files.iter()
            // .flat_map(|(_, f)| f.paragraphs.iter().map(|p| p as &dyn Linking).chain(f.headings.iter().map(|h| h as &dyn Linking)))
            .map(|(_, f)| f as &dyn Linking)
            .collect_vec()
    }
}

pub (super) trait Linking {
    fn get_links(&self) -> &Vec<Link>;
}



#[derive(Debug)]
pub struct MDFile {
    pub path: PathBuf,
    pub source: Vec<u8>,
    pub headings: Vec<MDHeading>,
    // pub paragraphs: Vec<MDParagraph>,
    pub tags: Vec<MDTag>,
    pub links: Vec<Link>,
    pub indexed_blocks: Vec<MDIndexedBlock>
}

impl MDFile {
    /// Regex parsing the file after reading it from path
    pub fn new(path: &str) -> MDFile {

        let source = read(&path).unwrap();
        let text = String::from_utf8(source.clone()).unwrap();

        static HEADING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(#+ (.+))").unwrap());
        let headings: Vec<MDHeading> = HEADING_RE.captures_iter(&text)
            .flat_map(|c| c.get(1).and_then(|r| c.get(0).map(|z| (z, r))))
            .map(|(full_heading, heading_text)| {
                let links = Link::regex_new(heading_text.as_str(), path);
                let file_match = Match {
                    file: path.into(),
                    text: full_heading.as_str().into(),
                    start: full_heading.range().start,
                    end: full_heading.range().end
                };
                return MDHeading {
                    links,
                    file_match,
                    heading_text: heading_text.as_str().into()
                }
            })
            .collect_vec();

        static INDEXED_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\n|^)(?<paragraph>.+ (?<index>\^\w+))\n").unwrap());
        let indexed_blocks: Vec<MDIndexedBlock> = INDEXED_BLOCK_RE.captures_iter(&text)
            .flat_map(|c| c.name("index").and_then(|index| c.name("paragraph").map(|paragraph| (paragraph, index))))
            .map(|(paragraph, index)|  {
                MDIndexedBlock {
                    file_match: 
                    Match { 
                        file: path.into(), 
                        text: paragraph.as_str().into(), 
                        start: paragraph.range().start,
                        end: paragraph.range().end 
                    },
                    index: index.as_str().into() 
                }
            })
            .collect_vec();
        println!("File: {:?} indexed_blocks: {:?}", indexed_blocks, path);



        // let mut ppath = PathBuf::new();
        // ppath.push(Path::new(path));
        // let paragraphs = query_and_links(&ppath, &source, "(paragraph (_) @paragraph);")
        //     .into_iter()
        //     .map(|(text_match, links)| MDParagraph::new(text_match, links))
        //     .collect_vec();

        static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"#([\w\/]+)").unwrap());
        let tags = TAG_RE.captures_iter(&text)
            .flat_map(|c| c.get(1))
            .map(|m| MDTag{
                tag: m.as_str().into(),
                file_match: Match { file: path.into(), text: m.as_str().into(), start: m.range().start, end: m.range().end }
            })
            .collect_vec();


        let links = Link::regex_new(&text, path);

        return MDFile {
            path: path.into(),
            headings,
            source,
            tags,
            links,
            indexed_blocks
        };

    }
}

impl Linking for MDFile {
    fn get_links(&self) -> &Vec<Link> {
        &self.links
    }
}


#[derive(Debug, PartialEq)]
pub struct MDHeading {
    pub heading_text: String,
    pub links: Vec<Link>,
    pub file_match: Match
}

impl MDHeading {
    pub fn new(source_match: Match, links: Vec<Link>) -> MDHeading {
        let heading_text = source_match.text.trim_start().to_owned();
        return MDHeading { heading_text, links, file_match: source_match }
    }
}


#[derive(Debug, PartialEq)]
pub struct MDIndexedBlock {
    pub file_match: Match,
    pub index: String
}


#[derive(Debug, PartialEq)]
pub struct MDTag {
    pub tag: String,
    pub file_match: Match
}

impl MDTag {
    pub fn new(file_match: Match) -> MDTag {
        let tag = file_match.text[1..].to_string();
        return MDTag {
            file_match,
            tag
        }
    }
}

impl Linkable for MDTag {
    fn get_range(&self) -> (&usize, &usize) {
        (&self.file_match.start, &self.file_match.end)
    }
    fn get_link_ref_name(&self) -> &String {
        &self.tag
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Match {
    pub file: PathBuf,
    pub text: String,
    pub start: usize,
    pub end: usize
}

#[derive(Debug, PartialEq)]
pub struct Link {
    pub link_ref: String,
    pub link_match: Match
}

impl Link {
    fn new(link_match: Match) -> Link {
        return Link {
            link_ref: link_match.text.to_owned(),
            link_match,
        }
    }
    fn regex_new(text: &str, path: &str) -> Vec<Link> {
        static LINK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\[([.[^\[\]]]+)\]\]").unwrap()); // Add support for []() quotes
        return LINK_RE.captures_iter(text)
            .map(|c| c.get(1).unwrap())
            .map(|m| Link {
                link_ref: String::from(m.as_str()), 
                link_match: Match { 
                    file: path.into(), 
                    text: String::from(m.as_str()), 
                    start: m.range().start, 
                    end: m.range().end
                }
            })
            .collect_vec();
    }
}

impl Linking for MDHeading {
    fn get_links(&self) -> &Vec<Link> {
        &self.links
    }
}

pub (super) trait Linkable {
    fn get_range(&self) -> (&usize, &usize);
    fn get_link_ref_name(&self) -> &String;
}
