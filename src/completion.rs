use itertools::Itertools;
use tower_lsp::lsp_types::{CompletionResponse, CompletionParams, CompletionTriggerKind, CompletionContext, CompletionItem};

use crate::vault::{Vault, Referenceable};

pub fn get_completions(vault: &Vault, params: CompletionParams) -> Option<CompletionResponse> {
    return match params {
        CompletionParams { context: Some(CompletionContext { trigger_character: Some(character), .. }), .. } if character == "#" => {
            // Initial Tag completion
            let all_tags = vault.select_linkable_nodes()
                .into_iter()
                .filter(|referenceable| matches!(referenceable, Referenceable::Tag(_, _)));

            return Some(CompletionResponse::Array(
                all_tags.map(|tag| tag.get_refname(&vault.root_dir()).map(|root| CompletionItem {label: root, ..Default::default()})).flatten().unique_by(|c| c.label.to_owned()).collect_vec()
            ))
        },
        _ => None
    }
}
