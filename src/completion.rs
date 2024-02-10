use std::hash::{DefaultHasher, Hash, Hasher};

use cached::proc_macro::cached;
use itertools::Itertools;
use rayon::prelude::*;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, Documentation,
};

use crate::{
    ui::preview_referenceable,
    vault::{Preview, Referenceable, Vault},
};

pub fn get_completions(vault: &Vault, params: &CompletionParams) -> Option<CompletionResponse> {
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
        == Some(&['[', '['])
    {
        // we have a link
        get_link_completions(vault)
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

#[cached(
    key = "u64",
    convert = r#"{ {
        calculate_hash(vault)
    } }"#
)] // TODO: this is stupid because it should know how to calculate the hash in a better way
fn get_link_completions(vault: &Vault) -> Option<tower_lsp::lsp_types::CompletionResponse> {
    let all_links = vault
        .select_referenceable_nodes(None)
        .into_iter()
        .filter(|referenceable| {
            !matches!(referenceable, Referenceable::Tag(_, _))
                && !matches!(referenceable, Referenceable::Footnote(_, _))
        });

    return Some(CompletionResponse::Array(
        all_links
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
                                .map(|text| root + &text),
                            _ => None,
                        },
                        ..Default::default()
                    })
            })
            .unique_by(|completion| completion.label.clone())
            .collect::<Vec<_>>(),
    ));
}
