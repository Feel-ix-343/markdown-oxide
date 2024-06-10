use std::path::Path;

use tower_lsp::lsp_types::{Hover, HoverContents, HoverParams};

use crate::{
    ui::{preview_reference, preview_referenceable},
};

use vault::Vault;

pub fn hover(vault: &Vault, params: &HoverParams, path: &Path) -> Option<Hover> {
    let cursor_position = params.text_document_position_params.position;

    match (
        vault.select_reference_at_position(path, cursor_position),
        vault.select_referenceable_at_position(path, cursor_position),
    ) {
        (Some(reference), _) => preview_reference(vault, path, reference).map(|markup| Hover {
            contents: HoverContents::Markup(markup),
            range: None,
        }),
        (None, Some(referenceable)) => {
            preview_referenceable(vault, &referenceable).map(|markup| Hover {
                contents: HoverContents::Markup(markup),
                range: None,
            })
        }
        _ => None,
    }
}
