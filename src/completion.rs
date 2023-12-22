use itertools::Itertools;
use tower_lsp::lsp_types::{CompletionResponse, CompletionParams, CompletionContext, CompletionItem, CompletionItemKind};

use crate::vault::{Vault, Referenceable};

pub fn get_completions(vault: &Vault, params: &CompletionParams) -> Option<CompletionResponse> {
    match params {
        CompletionParams { context: Some(CompletionContext { trigger_character: Some(character), .. }), .. } if character == "#" => {
            // Initial Tag completion
            let all_tags = vault.select_linkable_nodes()
                .into_iter()
                .filter(|referenceable| matches!(referenceable, Referenceable::Tag(_, _)));

            return Some(CompletionResponse::Array(
                all_tags.map(|tag| tag.get_refname(&vault.root_dir()).map(|root| CompletionItem {label: root, ..Default::default()})).flatten().unique_by(|c| c.label.to_owned()).collect_vec()
            ))
        },
        CompletionParams { text_document_position, .. } => {

            let Ok(path) = text_document_position.text_document.uri.to_file_path() else {
                return None;
            };

            let line = text_document_position.position.line as usize;
            let character = text_document_position.position.character as usize;

            let selected_line = vault.select_line(&path.to_path_buf(), line)?;

            if selected_line.get(character-2..character) == Some(&vec!['[', '[']) { // we have a link

                let all_tags = vault.select_linkable_nodes()
                    .into_iter()
                    .filter(|referenceable| !matches!(referenceable, Referenceable::Tag(_, _)));

                return Some(CompletionResponse::Array(
                    all_tags.map(|tag| tag.get_refname(&vault.root_dir()).map(|root| CompletionItem { kind: Some(CompletionItemKind::REFERENCE), label: root, ..Default::default()})).flatten().unique_by(|c| c.label.to_owned()).collect_vec()
                ))
            } else {
                return None
            }
        }
    }
}


