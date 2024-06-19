use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, CompletionTextEdit,
    Documentation, TextEdit,
};

use crate::{
    entity::{Entity, NamedEntityData, NamedEntityTypeInfo, UnnamedEntityData},
    entity_viewer::EntityViewer,
    parser::{QueryInfo, QuerySyntaxInfo, QuerySyntaxTypeInfo},
    settings::SettingsAdapter,
    Context,
};

use rayon::prelude::*;

pub fn named_completion_response(
    cx: &Context,
    info: &QueryInfo,
    named_entities: impl rayon::iter::IndexedParallelIterator<Item = Entity<NamedEntityData>>,
) -> CompletionResponse {
    let items = named_entities
        .take(50)
        .enumerate()
        .flat_map(|(i, entity)| {
            Some(CompletionItem {
                label: label(&entity),
                sort_text: Some(i.to_string()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit(
                    cx.settings(),
                    info,
                    &entity,
                ))),
                filter_text: Some(filter_text(info, &entity)),
                kind: Some(icon(&entity)),
                documentation: documentation(cx.entity_viewer(), &entity),
                ..Default::default()
            })
        })
        .collect::<Vec<_>>();

    CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items,
    })
}

fn icon(named_entity: &Entity<NamedEntityData>) -> CompletionItemKind {
    match named_entity.info().type_info {
        NamedEntityTypeInfo::File => CompletionItemKind::FILE,
        NamedEntityTypeInfo::Heading(..) | NamedEntityTypeInfo::IndexedBlock(..) => {
            CompletionItemKind::REFERENCE
        }
    }
}

fn named_entity_file_ref(entity: &Entity<NamedEntityData>) -> String {
    entity
        .info()
        .path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

fn label(entity: &Entity<NamedEntityData>) -> String {
    let file_ref = named_entity_file_ref(entity); // TODO: abstract this better; there is possible duplication in querier
    match entity.info() {
        NamedEntityData {
            path: _,
            type_info: NamedEntityTypeInfo::File,
        } => file_ref.to_string(),
        NamedEntityData {
            path: _,
            type_info: NamedEntityTypeInfo::Heading(heading),
        } => format!("{file_ref}#{heading}"),
        NamedEntityData {
            path: _,
            type_info: NamedEntityTypeInfo::IndexedBlock(index),
        } => format!("{file_ref}#^{index}"),
    }
}

// TODO: you should be able to simplify this by getting the total range and the query.
/// Text is filtered based on the range of the text edit being made This inserts the query label
/// into the relevant location while sending the user entered text for the rest of the range.
fn filter_text(info: &QueryInfo, named_entity: &Entity<NamedEntityData>) -> String {
    match info.query_syntax_info {
        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Wiki { display: _ },
        } => format!("[[{}", label(named_entity)),

        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Markdown { display: "" },
        } => format!("[]({}", label(named_entity)),

        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Markdown { display },
        } => format!("[{display}]({}", label(named_entity)),
    }
}

/// This is label for now, but when we consider file extensions, this will change
fn entity_ref(named_entity: &Entity<NamedEntityData>) -> String {
    label(named_entity)
}

fn text_edit(
    settings: &SettingsAdapter,
    info: &QueryInfo,
    named_entity: &Entity<NamedEntityData>,
) -> TextEdit {
    let entity_ref = entity_ref(named_entity);
    let new_text = match &info.query_syntax_info {
        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Markdown { display },
        } => {
            let wrapped_ref = match (
                entity_ref.clone(),
                entity_ref.contains(" "),
                settings.include_md_extension(),
            ) {
                (it, true, false) => format!("<{it}>"),
                (it, true, true) => format!("<{it}.md>"),
                (it, false, true) => format!("{it}.md"),
                (it, false, false) => it,
            };

            match (display, &named_entity.info().type_info) {
                (&"", NamedEntityTypeInfo::File | NamedEntityTypeInfo::IndexedBlock(..)) => {
                    format!("[]({wrapped_ref})")
                }
                (&"", NamedEntityTypeInfo::Heading(heading)) => {
                    format!("[{heading}]({wrapped_ref})")
                }
                (display, _) => format!("[{display}]({wrapped_ref})"),
            }
        }
        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Wiki { display },
        } => {
            let wrapped_ref = match settings.include_md_extension() {
                true => format!("{entity_ref}.md"),
                false => entity_ref.clone(),
            };
            match display {
                None => format!("[[{wrapped_ref}]]"),
                Some(display) => format!("[[{wrapped_ref}|{display}]]"),
            }
        }
    };

    let range = tower_lsp::lsp_types::Range {
        start: tower_lsp::lsp_types::Position {
            line: info.line as u32,
            character: info.char_range.start as u32,
        },
        end: tower_lsp::lsp_types::Position {
            line: info.line as u32,
            character: info.char_range.end as u32,
        },
    };

    TextEdit { range, new_text }
}

fn documentation(
    viewer: &EntityViewer,
    named_entity: &Entity<NamedEntityData>,
) -> Option<Documentation> {
    let text = viewer.entity_view(named_entity)?;

    Some(Documentation::MarkupContent(
        tower_lsp::lsp_types::MarkupContent {
            kind: tower_lsp::lsp_types::MarkupKind::Markdown,
            value: text,
        },
    ))
}

pub fn unnamed_completion_response(
    cx: &Context,
    info: &QueryInfo,
    named_entities: impl rayon::iter::IndexedParallelIterator<Item = Entity<UnnamedEntityData>>,
) -> CompletionResponse {
    todo!()
}
