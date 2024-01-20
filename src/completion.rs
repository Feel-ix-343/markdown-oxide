use itertools::Itertools;
use tower_lsp::lsp_types::{CompletionResponse, CompletionParams, CompletionItem, CompletionItemKind, Documentation};
use rayon::prelude::*;

use crate::{vault::{Vault, Referenceable}, ui::{preview_reference, preview_referenceable}};

pub fn get_completions(vault: &Vault, params: &CompletionParams) -> Option<CompletionResponse> {
    let Ok(path) = params.text_document_position.text_document.uri.to_file_path() else {
        return None;
    };

    let line = params.text_document_position.position.line as usize;
    let character = params.text_document_position.position.character as usize;

    let selected_line = vault.select_line(&path.to_path_buf(), line)?;

    if character.checked_sub(2).and_then(|start| selected_line.get(start..character)) == Some(&vec!['[', '[']) { // we have a link

        let all_links = vault.select_referenceable_nodes(None)
            .into_par_iter()
            .filter(|referenceable| !matches!(referenceable, Referenceable::Tag(_, _)) && !matches!(referenceable, Referenceable::Footnote(_, _)));

        return Some(
            CompletionResponse::Array(
                all_links
                    .map(|referenceable| referenceable.get_refname(&vault.root_dir())
                        .map(|root| CompletionItem { 
                            kind: Some(CompletionItemKind::FILE), 
                            label: root.clone(), 
                            documentation: preview_referenceable(vault, &referenceable).and_then(|markup| Some(Documentation::MarkupContent(markup))),
                            filter_text: match referenceable{
                                Referenceable::IndexedBlock(_, _) => vault.select_referenceable_preview(&referenceable).map(|text| root + &text),
                                _ => None
                            },
                            ..Default::default()
                        }))
                    .flatten()
                    .collect::<Vec<_>>()
        ))
    } else if character.checked_sub(2).and_then(|start| selected_line.get(start..character)) == Some(&vec!['#']) {

        // Initial Tag completion
        let tag_refereneables = vault.select_referenceable_nodes(None)
            .into_iter()
            .flat_map(|referenceable| match referenceable {
                tag @ Referenceable::Tag(_, _) => Some(tag),
                _ => None
            });


        return Some(CompletionResponse::Array(
            tag_refereneables
                .map(|tag| tag.get_refname(&vault.root_dir()).map(|root| CompletionItem { kind: Some(CompletionItemKind::CONSTANT), label: root, ..Default::default()})).flatten().unique_by(|c| c.label.to_owned()).collect_vec()
        )
        )
    } else if selected_line.get(character-1..character) == Some(&vec!['[']) {
        let footnote_referenceables = vault.select_referenceable_nodes(Some(&path))
            .into_iter()
            .flat_map(|referenceable| match referenceable {
                Referenceable::Footnote(footnote_path, _) if footnote_path.as_path() == path.as_path() => Some(referenceable),
                _ => None
            });


        return Some(CompletionResponse::Array(
            footnote_referenceables
                .map(|footnote| footnote.get_refname(&vault.root_dir())
                    .map(|root| CompletionItem { 
                        kind: Some(CompletionItemKind::REFERENCE), 
                        label: root.clone(), 
                        documentation: preview_referenceable(vault, &footnote).and_then(|markup| Some(Documentation::MarkupContent(markup))),
                        filter_text: vault.select_referenceable_preview(&footnote).map(|preview_string| root + &preview_string),
                        ..Default::default()
                    }))
                .flatten()
                .unique_by(|c| c.label.to_owned())
                .collect_vec()
        ))
    } else {
        return None
    }
}


