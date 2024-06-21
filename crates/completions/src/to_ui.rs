use tower_lsp::lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionList, CompletionResponse,
    CompletionTextEdit, Documentation, TextEdit, Url,
};

use nanoid::nanoid;

use crate::{
    entity::{Block, Entity, EntityInfo, NamedEntityTypeInfo},
    entity_viewer::EntityViewer,
    parser::{QueryInfo, QuerySyntaxInfo, QuerySyntaxTypeInfo},
    settings::SettingsAdapter,
    Context,
};

use rayon::prelude::*;

fn filter_text(info: &QueryInfo, adjusted_label: &str) -> String {
    match info.query_syntax_info {
        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Wiki { display: _ },
        } => format!("[[{}", adjusted_label),

        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Markdown { display: "" },
        } => format!("[]({}", adjusted_label),

        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Markdown { display },
        } => format!("[{display}]({}", adjusted_label),
    }
}

pub fn named_completion_response(
    cx: &Context,
    info: &QueryInfo,
    named_entities: impl rayon::iter::IndexedParallelIterator<Item = Entity>,
) -> CompletionResponse {
    let items = named_entities
        .take(50)
        .enumerate()
        .flat_map(|(i, entity)| {
            Some(CompletionItem {
                label: named_label(&entity),
                sort_text: Some(i.to_string()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit(
                    cx.settings(),
                    info,
                    &named_entity_ref(&entity),
                    Some(&named_to_md_link(&entity)),
                ))),
                filter_text: Some(filter_text(info, &named_label(&entity))),
                kind: Some(named_icon(&entity)),
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

fn named_label(entity: &Entity) -> String {
    let file_ref = named_entity_file_ref(entity); // TODO: abstract this better; there is possible duplication in querier
    match entity.info {
        EntityInfo {
            path: _,
            type_info: NamedEntityTypeInfo::File,
        } => file_ref.to_string(),
        EntityInfo {
            path: _,
            type_info: NamedEntityTypeInfo::Heading(heading),
        } => format!("{file_ref}#{heading}"),
        EntityInfo {
            path: _,
            type_info: NamedEntityTypeInfo::IndexedBlock(index),
        } => format!("{file_ref}#^{index}"),
    }
}

fn named_icon(named_entity: &Entity) -> CompletionItemKind {
    match named_entity.info.type_info {
        NamedEntityTypeInfo::File => CompletionItemKind::FILE,
        NamedEntityTypeInfo::Heading(..) | NamedEntityTypeInfo::IndexedBlock(..) => {
            CompletionItemKind::REFERENCE
        }
    }
}

fn named_entity_file_ref(entity: &Entity) -> String {
    entity
        .info
        .path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

/// This is label for now, but when we consider file extensions, this will change
fn named_entity_ref(named_entity: &Entity) -> String {
    named_label(named_entity)
}

fn named_to_md_link<'a>(entity: &'a Entity) -> impl Fn(MDDisplay, WrappedEntityRef) -> String + 'a {
    move |display: MDDisplay, wrapped_ref: WrappedEntityRef| match (display, &entity.info.type_info)
    {
        ("", NamedEntityTypeInfo::File | NamedEntityTypeInfo::IndexedBlock(..)) => {
            format!("[]({wrapped_ref})")
        }
        ("", NamedEntityTypeInfo::Heading(heading)) => {
            format!("[{heading}]({wrapped_ref})")
        }
        (display, _) => format!("[{display}]({wrapped_ref})"),
    }
}

type MDDisplay<'a> = &'a str;
type WrappedEntityRef<'a> = &'a str;
type ToMDLink<'a> = Option<&'a dyn Fn(MDDisplay, WrappedEntityRef) -> String>;

fn text_edit(
    settings: &SettingsAdapter,
    info: &QueryInfo,
    entity_ref: &str,
    to_md_link: ToMDLink,
) -> TextEdit {
    let new_text = match &info.query_syntax_info {
        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Markdown { display },
        } => {
            let wrapped_ref = match (
                entity_ref,
                entity_ref.contains(" "),
                settings.include_md_extension(),
            ) {
                (it, true, false) => format!("<{it}>"),
                (it, true, true) => format!("<{it}.md>"),
                (it, false, true) => format!("{it}.md"),
                (it, false, false) => it.to_string(),
            };

            match to_md_link {
                Some(func) => func(display, &wrapped_ref),
                None => format!("[{display}]({wrapped_ref})"),
            }
        }
        QuerySyntaxInfo {
            syntax_type_info: QuerySyntaxTypeInfo::Wiki { display },
        } => {
            let wrapped_ref = match settings.include_md_extension() {
                true => format!("{entity_ref}.md"),
                false => entity_ref.to_string(),
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

fn documentation(viewer: &EntityViewer, named_entity: &Entity) -> Option<Documentation> {
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
    unnamed_entities: impl rayon::iter::IndexedParallelIterator<Item = Block>,
) -> CompletionResponse {
    let items = unnamed_entities
        .take(50)
        .enumerate()
        .filter(|(_, it)| it.location_info().2 != info.path || it.location_info().0 != info.line)
        .flat_map(|(i, it)| {
            let optional_index = cx.querier().indexed_block_info(&it);

            let rand_index = nanoid!(
                5,
                &['a', 'b', 'c', 'd', 'e', 'f', 'g', '1', '2', '3', '4', '5', '6', '7', '8', '9']
            );

            Some(CompletionItem {
                label: unnamed_label(&it),
                filter_text: Some(filter_text(info, &format!(" {}", unnamed_label(&it)))),
                text_edit: Some(CompletionTextEdit::Edit(text_edit(
                    cx.settings(),
                    info,
                    &format!(
                        "{}#^{}",
                        it.location_info().2.file_stem().unwrap().to_str().unwrap(),
                        match &optional_index {
                            Some(index) => index.clone(),
                            None => rand_index.clone(),
                        }
                    ),
                    None,
                ))),
                sort_text: Some(i.to_string()),
                kind: Some(match optional_index {
                    Some(_) => CompletionItemKind::REFERENCE,
                    None => CompletionItemKind::TEXT,
                }),
                command: if optional_index.is_some() {
                    None
                } else {
                    Some(Command {
                        title: "Insert Block Reference Into File".into(),
                        command: "apply_edits".into(),
                        arguments: Some(vec![serde_json::to_value(
                            tower_lsp::lsp_types::WorkspaceEdit {
                                changes: Some(
                                    vec![(
                                        Url::from_file_path(it.location_info().2).expect(""),
                                        vec![TextEdit {
                                            range: tower_lsp::lsp_types::Range {
                                                start: tower_lsp::lsp_types::Position {
                                                    line: it.location_info().0 as u32,
                                                    character: it.location_info().1 as u32 - 1,
                                                },
                                                end: tower_lsp::lsp_types::Position {
                                                    line: it.location_info().0 as u32,
                                                    character: it.location_info().1 as u32 - 1,
                                                },
                                            },
                                            new_text: format!("   ^{}", rand_index),
                                        }],
                                    )]
                                    .into_iter()
                                    .collect(),
                                ),
                                change_annotations: None,
                                document_changes: None,
                            },
                        )
                        .ok()?]),
                    })
                },
                ..Default::default()
            })
        });

    CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items: items.collect::<Vec<_>>(),
    })
}

fn unnamed_label(entity: &Block) -> String {
    entity.info.line_text.to_string()
}
