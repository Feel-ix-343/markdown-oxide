use std::{hash::{DefaultHasher, Hash, Hasher}, path::{PathBuf, Path}};

use cached::proc_macro::cached;
use itertools::Itertools;
use rayon::prelude::*;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, Documentation, CompletionTextEdit, TextEdit, Range, Position,
};

use crate::{
    ui::preview_referenceable,
    vault::{Preview, Referenceable, Vault},
};

pub fn get_completions(vault: &Vault, opened_files: Vec<PathBuf>, params: &CompletionParams) -> Option<CompletionResponse> {
    let Ok(path) = params
        .text_document_position
        .text_document
        .uri
        .to_file_path()
    else {
        return None;
    };

    let line = params.text_document_position.position.line as usize;
    let character = params.text_document_position.position.character as usize;

    let selected_line = vault.select_line(&path.to_path_buf(), line)?;


    if character
        .checked_sub(2)
        .and_then(|start| selected_line.get(start..character))
    == Some(&['[', '[']) {

        Some(CompletionResponse::Array(opened_files
            .iter()
            .filter_map(|path| {
                Some(vault
                .select_referenceable_nodes(Some(path))
                .into_iter()
                    .filter(|referenceable| {
                        !matches!(referenceable, Referenceable::Tag(_, _))
                        && !matches!(referenceable, Referenceable::Footnote(_, _))
                    })
                    .collect_vec()
                )})
            .flatten()
            .filter_map(|referenceable| {
                referenceable
                    .get_refname(vault.root_dir())
                    .map(|root| CompletionItem {
                        kind: Some(CompletionItemKind::FILE),
                        label: root.clone(),
                        label_details: match referenceable.is_unresolved() {
                            true => Some(CompletionItemLabelDetails {
                                detail: Some("Unresolved".into()),
                                description: None,
                            }),
                            false => None,
                        },
                        documentation: preview_referenceable(vault, &referenceable)
                            .map(Documentation::MarkupContent),
                        filter_text: match referenceable {
                            Referenceable::IndexedBlock(_, _) => vault
                                .select_referenceable_preview(&referenceable)
                                .and_then(|preview| match preview {
                                    Preview::Text(string) => Some(string),
                                    Preview::Empty => None,
                                })
                                .map(|text| format!("{}{}", root, &text)),
                            _ => None,
                        },
                        ..Default::default()
                    })
            })
            .collect::<Vec<_>>()
        ))

    } else if character
        .checked_sub(3)
        .and_then(|start| selected_line.get(start..character-1))
    == Some(&['[', '['])
    {
        // we have a link



        // let all_links = get_links(vault)?;


        let all_links = vault
            .select_referenceable_nodes(None)
            .into_par_iter()
            .filter(|referenceable| {
                !matches!(referenceable, Referenceable::Tag(_, _))
                && !matches!(referenceable, Referenceable::Footnote(_, _))
            });



        let range = Range {
            start: Position {line: line as u32, character: (character - 1) as u32},
            end: Position {line: line as u32, character: character as u32}
        };

        let filter_char = &selected_line[character - 1..character][0];


        return Some(CompletionResponse::Array(

            all_links
                .filter(|referenceable| referenceable.get_refname(&vault.root_dir()).map(|name| name.to_lowercase().contains(filter_char.to_ascii_lowercase())) == Some(true))
                .filter_map(|referenceable| {
                    referenceable
                        .get_refname(vault.root_dir())
                        .map(|root| CompletionItem {
                            kind: Some(CompletionItemKind::FILE),
                            label: root.clone(),
                            label_details: match referenceable.is_unresolved() {
                                true => Some(CompletionItemLabelDetails {
                                    detail: Some("Unresolved".into()),
                                    description: None,
                                }),
                                false => None,
                            },
                            documentation: preview_referenceable(vault, &referenceable)
                                .map(Documentation::MarkupContent),
                            filter_text: match referenceable {
                                Referenceable::IndexedBlock(_, _) => vault
                                    .select_referenceable_preview(&referenceable)
                                    .and_then(|preview| match preview {
                                        Preview::Text(string) => Some(string),
                                        Preview::Empty => None,
                                    })
                                    .map(|text| format!("{}{}", root, &text)),
                                _ => None,
                            },
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                range,
                                new_text: root.clone()
                            })),
                            ..Default::default()
                        })
                })
                .collect::<Vec<_>>(),

        ));

    } else if character
        .checked_sub(1)
        .and_then(|start| selected_line.get(start..character))
    == Some(&['#'])
    {
        // Initial Tag completion
        let tag_refereneables =
        vault
            .select_referenceable_nodes(None)
            .into_iter()
            .flat_map(|referenceable| match referenceable {
                tag @ Referenceable::Tag(_, _) => Some(tag),
                _ => None,
            });

        return Some(CompletionResponse::Array(
            tag_refereneables
                .filter_map(|tag| {
                    tag.get_refname(vault.root_dir())
                        .map(|root| CompletionItem {
                            kind: Some(CompletionItemKind::CONSTANT),
                            label: root,
                            ..Default::default()
                        })
                })
                .unique_by(|c| c.label.to_owned())
                .collect_vec(),
        ));
    } else if character
        .checked_sub(1)
        .and_then(|start| selected_line.get(start..character))
    == Some(&['['])
    {
        let footnote_referenceables = vault
            .select_referenceable_nodes(Some(&path))
            .into_iter()
            .flat_map(|referenceable| match referenceable {
                Referenceable::Footnote(footnote_path, _)
                    if footnote_path.as_path() == path.as_path() =>
                    {
                        Some(referenceable)
                    }
                _ => None,
            });

        return Some(CompletionResponse::Array(
            footnote_referenceables
                .filter_map(|footnote| {
                    footnote
                        .get_refname(vault.root_dir())
                        .map(|root| CompletionItem {
                            kind: Some(CompletionItemKind::REFERENCE),
                            label: root.clone(),
                            documentation: preview_referenceable(vault, &footnote)
                                .map(Documentation::MarkupContent),
                            filter_text: vault
                                .select_referenceable_preview(&footnote)
                                .and_then(|preview| match preview {
                                    Preview::Text(string) => Some(string),
                                    Preview::Empty => None,
                                })
                                .map(|preview_string| root + &preview_string),
                            ..Default::default()
                        })
                })
                .unique_by(|c| c.label.to_owned())
                .collect_vec(),
        ));
    } else {
        return None;
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn get_links(vault: &Vault) -> Option<Vec<Referenceable>> {
    let re = vault
        .select_referenceable_nodes(None)
        .into_iter()
        .filter(|referenceable| {
            !matches!(referenceable, Referenceable::Tag(_, _))
            && !matches!(referenceable, Referenceable::Footnote(_, _))
        })
        .collect_vec();

    Some(re)
}
