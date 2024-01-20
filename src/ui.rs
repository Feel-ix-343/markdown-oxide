use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{MarkupContent, MarkupKind};

use crate::vault::{Vault, Reference, Referenceable};

fn referenceable_string(vault: &Vault, referenceable: &Referenceable) -> Option<String> {
    let preview = vault.select_referenceable_preview(referenceable)?;

    Some(match referenceable {
        Referenceable::File(_, _) => format!("File Preview:\n---\n\n{}", preview),
        Referenceable::Heading(_, _) => format!("Heading Preview:\n---\n\n{}", preview),
        Referenceable::IndexedBlock(_, _) => format!("Block Preview:\n---\n\n{}", preview),
        Referenceable::Footnote(_, _) => format!("Footnote Preview:\n---\n\n{}", preview),
        _ => format!("Preview:\n---\n\n{}", preview),
    })

}

pub fn preview_referenceable(vault: &Vault, referenceable: &Referenceable) -> Option<MarkupContent> {
    let display = referenceable_string(vault, referenceable)?;

    return Some(MarkupContent {
        kind: MarkupKind::Markdown,
        value: display
    })
}

pub fn preview_reference(vault: &Vault, reference_path: &Path, reference: &Reference) -> Option<MarkupContent> {
    match reference {
        Reference::Link(_) | Reference::Footnote(_) => {
            let positions = vault.select_referenceable_nodes(None);
            let referenceable = positions.iter().find(|i| i.matches_reference(&vault.root_dir(), &reference, &reference_path))?;

            let display = referenceable_string(vault, referenceable)?;

            return Some(MarkupContent {
                kind: MarkupKind::Markdown,
                value: display           
            })
        },
        _ => None
    }
}
