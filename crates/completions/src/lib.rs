#![feature(anonymous_lifetime_in_impl_trait)]

use std::{ops::{Deref, Range}, path::{Path, PathBuf}};

use rayon::prelude::*;
use regex::Regex;
use tower_lsp::lsp_types::{CompletionItem, CompletionList, CompletionParams, CompletionResponse};
use vault::Vault;
use moxide_config::Settings;


pub fn get_completions(
    vault: &Vault,
    _files: &Box<[PathBuf]>,
    params: &CompletionParams,
    path: &Path,
    _settings: &Settings
) -> Option<CompletionResponse> {

    let parser = Parser::new(vault);
    let querier = Querier::new(vault);
    let location = Location {
        path,
        line: params.text_document_position.position.line as usize,
        character: params.text_document_position.position.character as usize
    };

    dbg!(completions(&parser, &querier, location))
}

fn completions(parser: &Parser, querier: &Querier, location: Location) -> Option<CompletionResponse> {
    let (file_ref, link_info) = parser.parse_link(location)?;
    let files = querier.query(file_ref);
    Some(to_completion_response(&link_info, files.into_par_iter()))
}


struct Parser<'a> {
    vault: &'a Vault
}

impl<'a> Parser<'a> {
    fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}

impl Parser<'_> {
    fn parse_link(&self, location: Location) -> Option<(FileRef, LinkInfo)> {

        let chars = self.vault.select_line(location.path, location.line as isize)?;
        let line_string = String::from_iter(chars);

        let re = Regex::new(r"\[\[(?<file_ref>.*?)\]\]").expect("Regex failed to compile");

        let c = re.captures_iter(&line_string)
            .next()?;
        let file_ref = c.name("file_ref")?.as_str();
        let char_range = c.get(0)?.start()..c.get(0)?.end();

        Some((file_ref.to_string(), LinkInfo {char_range, line: location.line}))

    }
}

type FileRef = String;
struct LinkInfo {
    line: usize,
    char_range: Range<usize>
}

struct Location<'fs> {
    path: &'fs Path,
    line: usize,
    character: usize
}

struct Querier<'a> {
    vault: &'a Vault
}

impl<'a> Querier<'a> {
    fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}

impl<'a> Querier<'a> {
    fn query(&self, file_ref: FileRef) -> Vec<&'a Path> {
        let paths = self.vault.md_files
            .keys()
            .map(|key| (key.file_name().unwrap().to_str().unwrap().to_string(), key))
            .collect::<Vec<_>>();

        let matched = fuzzy_match(&file_ref, paths);
        matched.into_par_iter()
            .map(|((_, path), _)| path as &Path)
            .collect()
    }
}

impl<'a> Matchable for (String, &'a PathBuf) {
    fn match_string(&self) -> &str {
        self.0.as_str()
    }
}


pub trait Matchable {
    fn match_string(&self) -> &str;
}


struct NucleoMatchable<T: Matchable>(T);
impl<T: Matchable> Deref for NucleoMatchable<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Matchable> AsRef<str> for NucleoMatchable<T> {
    fn as_ref(&self) -> &str {
        self.match_string()
    }
}


use nucleo_matcher::{pattern::{self, Normalization}, Matcher};
pub fn fuzzy_match<'a, T: Matchable>(
    filter_text: &str,
    items: impl IntoIterator<Item = T>,
) -> Vec<(T, u32)> {
    let items = items.into_iter().map(NucleoMatchable);

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let matches = pattern::Pattern::parse(
        filter_text,
        pattern::CaseMatching::Smart,
        Normalization::Smart,
    )
        .match_list(items, &mut matcher);

    matches
        .into_iter()
        .map(|(item, score)| (item.0, score))
        .collect()
}




fn to_completion_response(info: &LinkInfo, files: impl IndexedParallelIterator<Item = &Path>) -> CompletionResponse {
    let items = files.enumerate()
        .flat_map(|(i, path)| Some((i, path.file_name()?.to_str()?)))
        .flat_map(|(i, name)| Some(CompletionItem {
            label: name.to_string(),
            sort_text: Some(i.to_string()),
            ..Default::default()
        }))
        .collect::<Vec<_>>();

    CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items
    })
}

