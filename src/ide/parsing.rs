use std::{path::{PathBuf, Path}, collections::HashMap, fs::{DirEntry, read}, ffi::OsString};

use itertools::Itertools;
use rayon::prelude::*;
use tree_sitter::{Query, QueryCursor, Tree, Language};
use tree_sitter_md::{MarkdownParser, language, inline_language};

pub (super) struct Vault {
    pub (super) files: HashMap<String, MDFile>
}

pub (super) fn get_parsed_vault(vault_dir: &str) -> Result<Vault, std::io::Error> {
    let dir_path = Path::new(vault_dir).to_owned();



    // The MDFiles by their ref name
    let md_files: HashMap<String, MDFile> = dir_path
        .read_dir()?
        .filter_map(|f| Result::ok(f))
        .collect_vec()
        .par_iter()
        .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
        .map(|f| {

            let md_file = MDFile::new(f.path());

            let relative_path = f.path().strip_prefix(&dir_path).unwrap().to_owned();
            let ref_name = relative_path.file_stem().unwrap().to_owned(); // TODO: Make sure that this did not mess up folders

            return (ref_name.to_str().unwrap().to_owned(), md_file)
        })
        .collect();

    return Ok(Vault { files: md_files })

}


#[derive(Debug)]
pub struct MDFile {
    pub path: PathBuf,
    pub source: Vec<u8>,
    pub headings: Vec<MDHeading>,
    pub paragraphs: Vec<MDParagraph>,
    pub tags: Vec<MDTag>
}

impl MDFile {
    pub fn new(path: PathBuf) -> MDFile {
        let source = read(&path).unwrap();

        let headings = query_and_links(&path, &source, "(atx_heading heading_content: (inline) @heading);")
            .into_iter()
            .map(|(text_match, links)| MDHeading::new(text_match, links))
            .collect_vec();

        let paragraphs = query_and_links(&path, &source, "(paragraph (_) @paragraph);")
            .into_iter()
            .map(|(text_match, links)| MDParagraph::new(text_match, links))
            .collect_vec();

        let tags = query_and_links(&path, &source, "(paragraph (_) @paragraph);")
            .into_iter()
            .filter(|(text_match, _)| text_match.text.starts_with("#"))
            .map(|(text_match, _)| MDTag::new(text_match))
            .collect_vec();

        return MDFile {
            path,
            headings,
            paragraphs,
            source,
            tags
        };

    }
}

#[derive(Debug, PartialEq)]
pub struct MDHeading {
    pub heading_text: String,
    pub resolved_links: Vec<Link>,
    pub file_match: Match
}

impl MDHeading {
    pub fn new(source_match: Match, links: Vec<Link>) -> MDHeading {
        let heading_text = source_match.text.trim_start().to_owned();
        return MDHeading { heading_text, resolved_links: links, file_match: source_match }
    }
}


#[derive(Debug, PartialEq)]
pub struct MDParagraph {
    pub resolved_links: Vec<Link>,
    pub file_match: Match
}

impl MDParagraph {
    pub fn new(source_match: Match, links: Vec<Link>) -> MDParagraph {
        MDParagraph { resolved_links: links, file_match: source_match }
    }
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

#[derive(Debug, PartialEq, Clone)]
pub struct Match {
    pub file: PathBuf,
    pub text: String,
    pub start: (usize, usize),
    pub end: (usize, usize)
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
}

pub fn query_and_links<'a>(file: &PathBuf, source_code: &'a [u8], query: &str) -> Vec<(Match, Vec<Link>)> {

    let query_matches: Vec<Match> = query_matches_block(file, &source_code, query);

    query_matches.into_iter()
        .map( |m| {
            // get the match text as an array of u8
            let match_text = m.text.as_bytes();
            let link_matches = query_matches_inline(&file, &match_text, "(link_text) @link;");

            let links: Vec<Link> = link_matches.into_iter()
                .map(Link::new)
                .collect_vec();

            return (m, links);
        })
        .collect_vec()
}

fn query_matches_block<'a>(file: &PathBuf, source_code: &'a [u8], query: &str) -> Vec<Match> {
    let mut parser = MarkdownParser::default();

    let tree = parser.parse(source_code, None).unwrap();
    // let language = language();
    let language = language();

    let block_tree = tree.block_tree();

    // Finding links in the files

    let mut query_cursor = QueryCursor::new();
    let text_provider: &[u8] = &[];

    let matches = tree_matches(&file, block_tree, language, query, source_code);

    // wow what a bad bug! Don't do this! All of the matches will have the ranges
    // let matches = query_cursor.matches(&query, block_tree.root_node(), text_provider).collect_vec();
    // println!("matches: {:?}", matches);

    return matches
}

fn query_matches_inline<'a>(file: &PathBuf, source_code: &'a [u8], query: &str) -> Vec<Match> {
    let mut parser = MarkdownParser::default();

    let tree = parser.parse(source_code, None).unwrap();
    // let language = language();
    let language = inline_language();

    let inline_trees = tree.inline_trees();

    // Finding links in the files

    let mut query_cursor = QueryCursor::new();
    let text_provider: &[u8] = &[];

    let matches = inline_trees.into_iter()
        .flat_map(|tree| {
            tree_matches(file, tree, language, query, source_code)
        })
        .collect_vec();


    // wow what a bad bug! Don't do this! All of the matches will have the ranges
    // let matches = query_cursor.matches(&query, block_tree.root_node(), text_provider).collect_vec();
    // println!("matches: {:?}", matches);

    return matches
}

fn tree_matches(file: &PathBuf, tree: &Tree, language: Language, query: &str, source_code: &[u8]) -> Vec<Match> {
    let query = Query::new(language, query).unwrap();
    let text_provider: &[u8] = &[];
    let mut query_cursor = QueryCursor::new();

    return query_cursor
        .matches(&query, tree.root_node(), text_provider)
        .flat_map(|m| m.captures)
        .map(|c| {
            Match {
                file: file.to_path_buf(),
                text: c.node.utf8_text(source_code).unwrap().to_owned(),
                start: (c.node.start_position().column, c.node.start_position().row),
                end: (c.node.end_position().column, c.node.end_position().row),
            }
        })
        .collect_vec();
}

