use std::path::PathBuf;
use super::vault_parser::{Match, Link};

#[derive(Debug)]
pub struct MDFile {
    pub path: PathBuf,
    pub source: Vec<u8>,
    pub headings: Vec<MDHeading>,
    pub paragraphs: Vec<MDParagraph>
}

#[derive(Debug)]
pub struct MDHeading {
    pub heading_text: String,
    resolved_links: Vec<Link>,
    file_match: Match
}

impl MDHeading {
    pub fn new(source_match: Match, links: Vec<Link>) -> MDHeading {
        let heading_text = source_match.text.trim_start().to_owned();
        return MDHeading { heading_text, resolved_links: links, file_match: source_match }
    }
}

#[derive(Debug)]
pub struct MDParagraph {
    resolved_links: Vec<Link>,
    file_match: Match
}

impl MDParagraph {
    pub fn new(source_match: Match, links: Vec<Link>) -> MDParagraph {
        MDParagraph { resolved_links: links, file_match: source_match }
    }
}

