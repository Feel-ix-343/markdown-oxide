use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{MarkupContent, MarkupKind};

use crate::vault::{Vault, Reference, Referenceable};

fn referenceable_string(vault: &Vault, referenceable: &Referenceable) -> Option<String> {
    let links_text = match referenceable {
        Referenceable::Footnote(_, _) => {
            let range = referenceable.get_range();
            String::from_iter(vault.select_line(&referenceable.get_path(), range.start.line as usize)?)
        },
        Referenceable::File(_, _) | Referenceable::Heading(_, _) | Referenceable::IndexedBlock(_, _) => {
            let range = referenceable.get_range();
            (range.start.line..=range.end.line + 10)
                .map(|ln| vault.select_line(&referenceable.get_path(), ln as usize))
                .flatten() // flatten those options!
                .map(|vec| String::from_iter(vec))
                .join("")
        }
        _ => return None
    }; 

    Some(match referenceable {
        Referenceable::File(_, _) => format!("File Preview:\n---\n\n{}", links_text),
        Referenceable::Heading(_, _) => format!("Heading Preview:\n---\n\n{}", links_text),
        Referenceable::IndexedBlock(_, _) => format!("Block Preview:\n---\n\n{}", links_text),
        Referenceable::Footnote(_, _) => format!("Footnote Preview:\n---\n\n{}", links_text),
        _ => format!("Preview:\n---\n\n{}", links_text),
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
            let referenceable = positions.iter().find(|i| i.is_reference(&vault.root_dir(), &reference, &reference_path))?;

            let display = referenceable_string(vault, referenceable)?;

            return Some(MarkupContent {
                kind: MarkupKind::Markdown,
                value: display           
            })
        },
        _ => None
    }
}
