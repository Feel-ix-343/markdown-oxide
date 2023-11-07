use std::{path::{PathBuf, Path}, collections::HashMap, fs::{DirEntry, read}, ffi::OsString};

use itertools::Itertools;
use rayon::prelude::*;
use tree_sitter::{Query, QueryCursor, Tree, Language};
use tree_sitter_md::{MarkdownParser, language, inline_language};

use super::nodes::{MDFile, MDHeading, MDParagraph};

pub fn parse_vault(dir: &str) -> Result<(), std::io::Error>  {

    let dir_path = Path::new(dir).to_owned();



    let md_files: HashMap<String, MDFile> = dir_path
        .read_dir()?
        .filter_map(|f| Result::ok(f))
        .collect_vec()
        .par_iter()
        .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
        .map(|f| {



            let relative_path = f.path().strip_prefix(&dir_path).unwrap().to_owned();
            let ref_name = relative_path.file_stem().unwrap().to_owned(); // TODO: Make sure that this did not mess up folders

            return (ref_name.to_str().unwrap().to_owned(), md_file)
        })
        .collect();

    // Map of all headings by obsidian style refname
    let headings: HashMap<String, &MDHeading> = md_files.iter()
        .flat_map(|(s, f)| {
            f.headings.iter().map(move |h| {
                let ref_name = format!("{}#{}", s, h.heading_text);
                    (ref_name, h)
            })
        })
        .collect();


    // TODO: Tags, lists, ... more specific thigns
    return Ok(())
}

#[derive(Debug)]
pub struct Match {

    pub text: String,
    pub start: (usize, usize),
    pub end: (usize, usize)
}

#[derive(Debug)]
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

fn query_and_links<'a>(source_code: &'a [u8], query: &str) -> Vec<(Match, Vec<Link>)> {

    let query_matches: Vec<Match> = query_matches_block(&source_code, query);

    query_matches.into_iter()
        .map( |m| {
            // get the match text as an array of u8
            let match_text = m.text.as_bytes();
            let link_matches = query_matches_inline(&match_text, "(link_text) @link;");

            let links: Vec<Link> = link_matches.into_iter()
                .map(Link::new)
                .collect_vec();

            return (m, links);
        })
        .collect_vec()
}

fn query_matches_block<'a>(source_code: &'a [u8], query: &str) -> Vec<Match> {
    let mut parser = MarkdownParser::default();

    let tree = parser.parse(source_code, None).unwrap();
    // let language = language();
    let language = language();

    let block_tree = tree.block_tree();

    // Finding links in the files

    let mut query_cursor = QueryCursor::new();
    let text_provider: &[u8] = &[];

    let matches = tree_matches(block_tree, language, query, source_code);

    // wow what a bad bug! Don't do this! All of the matches will have the ranges
    // let matches = query_cursor.matches(&query, block_tree.root_node(), text_provider).collect_vec();
    // println!("matches: {:?}", matches);

    return matches
}

fn query_matches_inline<'a>(source_code: &'a [u8], query: &str) -> Vec<Match> {
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
            tree_matches(tree, language, query, source_code)
        })
        .collect_vec();


    // wow what a bad bug! Don't do this! All of the matches will have the ranges
    // let matches = query_cursor.matches(&query, block_tree.root_node(), text_provider).collect_vec();
    // println!("matches: {:?}", matches);

    return matches
}

fn tree_matches(tree: &Tree, language: Language, query: &str, source_code: &[u8]) -> Vec<Match> {
    let query = Query::new(language, query).unwrap();
    let text_provider: &[u8] = &[];
    let mut query_cursor = QueryCursor::new();

    return query_cursor
        .matches(&query, tree.root_node(), text_provider)
        .flat_map(|m| m.captures)
        .map(|c| {
            Match {
                text: c.node.utf8_text(source_code).unwrap().to_owned(),
                start: (c.node.start_position().column, c.node.start_position().row),
                end: (c.node.end_position().column, c.node.end_position().row),
            }
        })
        .collect_vec();
}

