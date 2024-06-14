use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, CompletionTextEdit,
    TextEdit,
};

use crate::{
    parser::{QueryInfo, QuerySyntaxInfo, QuerySyntaxTypeInfo},
    querier::{NamedEntity, NamedEntityInfo},
};

use rayon::prelude::*;

pub fn completion_response(
    info: &QueryInfo,
    named_entities: impl rayon::iter::IndexedParallelIterator<Item = NamedEntity>,
) -> CompletionResponse {
    let items = named_entities
        .take(50)
        .enumerate()
        .flat_map(|(i, entity)| {
            Some(CompletionItem {
                label: label(&entity),
                sort_text: Some(i.to_string()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit(info, &entity))),
                filter_text: Some(filter_text(info, &entity)),
                kind: Some(icon(&entity)),
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

fn icon(named_entity: &NamedEntity) -> CompletionItemKind {
    match named_entity.1 {
        NamedEntityInfo::File => CompletionItemKind::FILE,
        NamedEntityInfo::Heading(..) | NamedEntityInfo::IndexedBlock(..) => {
            CompletionItemKind::REFERENCE
        }
    }
}

fn named_entity_file_ref(entity: &NamedEntity) -> String {
    entity.0.file_stem().unwrap().to_str().unwrap().to_string()
}

fn label(entity: &NamedEntity) -> String {
    let file_ref = named_entity_file_ref(entity); // TODO: abstract this better; there is possible duplication in querier
    match entity {
        NamedEntity(_, NamedEntityInfo::File) => file_ref.to_string(),
        NamedEntity(_, NamedEntityInfo::Heading(heading)) => format!("{file_ref}#{heading}"),
        NamedEntity(_, NamedEntityInfo::IndexedBlock(index)) => format!("{file_ref}#^{index}"),
    }
}

fn filter_text(info: &QueryInfo, named_entity: &NamedEntity) -> String {
    match info.query_syntax_info {
        QuerySyntaxInfo {
            display: _,
            syntax_type_info: QuerySyntaxTypeInfo::Wiki,
        } => format!("[[{}", label(named_entity)),

        QuerySyntaxInfo {
            display: None,
            syntax_type_info: QuerySyntaxTypeInfo::Markdown,
        } => format!("[]({}", label(named_entity)),

        QuerySyntaxInfo {
            display: Some(display),
            syntax_type_info: QuerySyntaxTypeInfo::Markdown,
        } => format!("[{display}]({}", label(named_entity)),
    }
}

/// This is label for now, but when we consider file extensions, this will change
fn entity_ref(named_entity: &NamedEntity) -> String {
    label(named_entity)
}

fn text_edit(info: &QueryInfo, named_entity: &NamedEntity) -> TextEdit {
    let entity_ref = entity_ref(named_entity);
    let new_text = match info.query_syntax_info {
        QuerySyntaxInfo {
            display: None,
            syntax_type_info: QuerySyntaxTypeInfo::Wiki,
        } => format!("[[{entity_ref}]]"),
        QuerySyntaxInfo {
            display: Some(display),
            syntax_type_info: QuerySyntaxTypeInfo::Wiki,
        } => format!("[[{entity_ref}|{display}]]"),
        QuerySyntaxInfo {
            display: None,
            syntax_type_info: QuerySyntaxTypeInfo::Markdown,
        } => format!("[]({entity_ref})"),
        QuerySyntaxInfo {
            display: Some(display),
            syntax_type_info: QuerySyntaxTypeInfo::Markdown,
        } => format!("[{display}]({entity_ref})"),
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
