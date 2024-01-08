use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{MarkupContent, MarkupKind};

use crate::vault::{Vault, Reference, Referenceable};

pub fn preview_reference(vault: &Vault, reference_path: &Path, reference: &Reference) -> Option<MarkupContent> {
    match reference {
        Reference::Link(_) => {
            let positions = vault.select_referenceable_nodes(None);
            let referenceable = positions.iter().find(|i| i.is_reference(&vault.root_dir(), &reference, &reference_path))?;


            let range = referenceable.get_range();
            let links_text: String = (range.start.line..=range.end.line + 10)
                .map(|ln| vault.select_line(&referenceable.get_path(), ln as usize))
                .flatten() // flatten those options!
                .map(|vec| String::from_iter(vec))
                .join("");

            return Some(MarkupContent {
                kind: MarkupKind::Markdown,
                value: match referenceable {
                    Referenceable::File(_, _) => format!("File Preview:\n---\n\n{}", links_text),
                    Referenceable::Heading(_, _) => format!("Heading Preview:\n---\n\n{}", links_text),
                    Referenceable::IndexedBlock(_, _) => format!("Block Preview:\n---\n\n{}", links_text),
                    _ => format!("Preview:\n---\n\n{}", links_text),
                } 
            })
        },
        Reference::Footnote(_) => {
            let positions = vault.select_referenceable_nodes(None);
            let referenceable = positions.iter().find(|i| i.is_reference(&vault.root_dir(), &reference, &reference_path))?;

            let range = referenceable.get_range();
            let text: String = String::from_iter(vault.select_line(&referenceable.get_path(), range.start.line as usize)?);

            return Some(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("Footnote Preview:\n---\n\n{}", text),
            })
        },
        _ => None
    }
}
