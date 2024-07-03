#![allow(clippy::map_flatten)]
use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{MarkupContent, MarkupKind};

use vault::{get_obsidian_ref_path, Preview, Reference, Referenceable, Vault};

pub fn referenceable_string(vault: &Vault, referenceables: &[Referenceable]) -> Option<String> {
    let referenceable = referenceables.first()?;

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

    let backlinks_preview = match referenceables
        .iter()
        .flat_map(|i| vault.select_references_for_referenceable(i))
        .flatten()
        .collect_vec()
    {
        references if !references.is_empty() => references
            .into_iter()
            .take(20)
            .flat_map(|(path, reference)| {
                let line = String::from_iter(
                    vault.select_line(path, reference.data().range.start.line as isize)?,
                );

                let path = get_obsidian_ref_path(vault.root_dir(), path)?;

                Some(format!("- `{}`: `{}`", path, line)) // and select indented list
            })
            .join("\n"),
        _ => "No Backlinks".to_string(),
    };

    Some(format!(
        "{}\n\n`...`\n\n---\n\n# Backlinks\n\n{}",
        written_text_preview, backlinks_preview
    ))
}

pub fn preview_referenceable(
    vault: &Vault,
    referenceable: &Referenceable,
) -> Option<MarkupContent> {
    let display = referenceable_string(vault, &[referenceable.clone()])?;

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
        WikiFileLink(..)
        | WikiHeadingLink(..)
        | WikiIndexedBlockLink(..)
        | Footnote(_)
        | MDFileLink(..)
        | MDHeadingLink(..)
        | MDIndexedBlockLink(..)
        | LinkRef(..) => {
            let referenceables_for_reference =
                vault.select_referenceables_for_reference(reference, reference_path);

            let display = referenceable_string(vault, &referenceables_for_reference)?;

            Some(MarkupContent {
                kind: MarkupKind::Markdown,
                value: display,
            })
        }
        Tag(_) => None,
    }
}
