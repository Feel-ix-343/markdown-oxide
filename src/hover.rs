use std::path::Path;

use tower_lsp::lsp_types::{Hover, HoverContents, HoverParams};

use crate::{config::Settings, ui::preview_reference, vault::Vault};

pub fn hover(
    vault: &Vault,
    params: &HoverParams,
    path: &Path,
    settings: &Settings,
) -> Option<Hover> {
    if !settings.hover {
        return None;
    }

    let cursor_position = params.text_document_position_params.position;

    match (
        vault.select_reference_at_position(path, cursor_position),
        vault.select_referenceable_at_position(path, cursor_position),
    ) {
        (Some(reference), _) => preview_reference(vault, path, reference).map(|markup| Hover {
            contents: HoverContents::Markup(markup),
            range: None,
        }),
        _ => None,
    }
}
