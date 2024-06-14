#![feature(anonymous_lifetime_in_impl_trait)]

mod parser;
mod querier;
mod to_lsp;

use std::path::{Path, PathBuf};

use moxide_config::Settings;
use parser::Parser;
use querier::Querier;
use to_lsp::completion_response;
use tower_lsp::lsp_types::{CompletionParams, CompletionResponse};
use vault::Vault;

pub fn get_completions(
    vault: &Vault,
    _files: &[PathBuf],
    params: &CompletionParams,
    path: &Path,
    _settings: &Settings,
) -> Option<CompletionResponse> {
    let parser = Parser::new(vault);
    let querier = Querier::new(vault);
    let location = Location {
        path,
        line: params.text_document_position.position.line as usize,
        character: params.text_document_position.position.character as usize,
    };

    completions(&parser, &querier, location)
}

fn completions(
    parser: &Parser,
    querier: &Querier,
    location: Location,
) -> Option<CompletionResponse> {
    let (named_entity_query, query_syntax_info) = parser.parse_link(location)?;
    let named_entities = querier.query(named_entity_query);
    Some(completion_response(&query_syntax_info, named_entities))
}

struct Location<'fs> {
    path: &'fs Path,
    line: usize,
    character: usize,
}
