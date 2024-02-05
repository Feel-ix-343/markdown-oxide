use std::path::Path;

use tower_lsp::lsp_types::{MarkupContent, MarkupKind};

use crate::vault::{Reference, Referenceable, Vault};

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

pub fn preview_referenceable(
    vault: &Vault,
    referenceable: &Referenceable,
) -> Option<MarkupContent> {
    let display = referenceable_string(vault, referenceable)?;

    Some(MarkupContent {
        kind: MarkupKind::Markdown,
        value: display,
    })
}

use Reference::*;

pub fn preview_reference(
    vault: &Vault,
    reference_path: &Path,
    reference: &Reference,
) -> Option<MarkupContent> {
    match reference {
        FileLink(..) | HeadingLink(..) | IndexedBlockLink(..) | Footnote(_) => {
            let positions = vault.select_referenceable_nodes(None);
            let referenceable = positions
                .iter()
                .find(|i| reference.references(&vault.root_dir(), i.get_path(), i))?;

            let display = referenceable_string(vault, referenceable)?;

            Some(MarkupContent {
                kind: MarkupKind::Markdown,
                value: display,
            })
        }
        _ => None,
    }
}
