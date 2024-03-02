use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{MarkupContent, MarkupKind};

use crate::vault::{Preview, Reference, Referenceable, Vault, get_obsidian_ref_path};

fn referenceable_string(vault: &Vault, referenceable: &Referenceable) -> Option<String> {
    let preview = vault.select_referenceable_preview(referenceable);

    let written_text_preview = match preview {
        Some(Preview::Empty) => "No Text".into(),
        Some(Preview::Text(text)) => match referenceable {
            Referenceable::File(_, _) => format!("`File Preview:`\n\n{}", text),
            Referenceable::Heading(_, _) => format!("`Heading Preview:`\n\n{}", text),
            Referenceable::IndexedBlock(_, _) => format!("`Block Preview:`\n\n{}", text),
            Referenceable::Footnote(_, _) => format!("`Footnote Preview:`\n\n{}", text),
            _ => format!("`Preview:`\n{}", text),
        },
        None => "No Preview".into(),
    };


    let backlinks_preview = match vault.select_references_for_referenceable(referenceable) {
        Some(references) => references.into_iter()
            .take(5)
            .flat_map(|(path, reference)| {
                let line = String::from_iter(vault.select_line(path, reference.data().range.start.line as isize)?);

                let path = get_obsidian_ref_path(&vault.root_dir(), path)?;

                Some(format!("- `{}`: `{}`", path, line)) // and select indented list
            })
            .join(""),
        None => format!("No Backlinks")
    };


    return Some(format!("{}\n\n`...`\n\n---\n\n# Backlinks\n\n{}", written_text_preview, backlinks_preview))
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
    _reference_path: &Path,
    reference: &Reference,
) -> Option<MarkupContent> {
    match reference {
        WikiFileLink(..) | WikiHeadingLink(..) | WikiIndexedBlockLink(..) | Footnote(_) | MDFileLink(..) | MDHeadingLink(..) | MDIndexedBlockLink(..) | LinkRef(..) => {
            let positions = vault.select_referenceable_nodes(None);
            let referenceable = positions
                .iter()
                .find(|i| reference.references(vault.root_dir(), i.get_path(), i))?;

            let display = referenceable_string(vault, referenceable)?;

            Some(MarkupContent {
                kind: MarkupKind::Markdown,
                value: display,
            })
        }
        Tag(_) => None,
    }
}
