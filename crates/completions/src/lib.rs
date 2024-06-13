#![feature(anonymous_lifetime_in_impl_trait)]

mod parser;
mod querier;

use std::{
    ops::{Deref, Range},
    path::{Path, PathBuf},
};

use moxide_config::Settings;
use parser::{LinkInfo, Parser};
use querier::{NamedEntity, NamedEntityInfo, Querier};
use rayon::prelude::*;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionList, CompletionParams, CompletionResponse, CompletionTextEdit,
    TextEdit,
};
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
    let (link, link_info) = parser.parse_link(location)?;

    let named_entities = querier.query(link);

    Some(to_completion_response(&link_info, named_entities))
}

struct Location<'fs> {
    path: &'fs Path,
    line: usize,
    character: usize,
}

fn to_completion_response(
    info: &LinkInfo,
    named_entities: impl IndexedParallelIterator<Item = NamedEntity>,
) -> CompletionResponse {
    let items = named_entities
        .take(20)
        .enumerate()
        .flat_map(|(i, entity)| {
            let label = entity_to_label(&entity);

            Some(CompletionItem {
                label: label.clone(),
                sort_text: Some(i.to_string()),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: tower_lsp::lsp_types::Range {
                        start: tower_lsp::lsp_types::Position {
                            line: info.line as u32,
                            character: info.char_range.start as u32,
                        },
                        end: tower_lsp::lsp_types::Position {
                            line: info.line as u32,
                            character: info.char_range.end as u32,
                        },
                    },
                    new_text: format!("[[{label}]]"),
                })),
                filter_text: Some(format!("[[{label}")),
                ..Default::default()
            })
        })
        .collect::<Vec<_>>();

    dbg!(&items);

    CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items,
    })
}

fn named_entity_file_ref(entity: &NamedEntity) -> String {
    entity.0.file_stem().unwrap().to_str().unwrap().to_string()
}

fn entity_to_label(entity: &NamedEntity) -> String {
    let file_ref = named_entity_file_ref(entity); // TODO: abstract this better; there is possible duplication in querier
    match entity {
        NamedEntity(_, querier::NamedEntityInfo::File) => file_ref.to_string(),
        NamedEntity(_, NamedEntityInfo::Heading(heading)) => format!("{file_ref}#{heading}"),
        NamedEntity(_, NamedEntityInfo::IndexedBlock(index)) => format!("{file_ref}#^{index}"),
    }
}
