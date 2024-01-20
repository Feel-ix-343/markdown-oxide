use std::path::Path;

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::vault::Vault;

pub fn references(vault: &Vault, cursor_position: Position, path: &Path) -> Option<Vec<Location>> {
    // First we need to get the referenceable node under the cursor
    let referenceable = vault.select_referenceable_at_position(path, cursor_position)?;

    Some(
        vault
            .select_references_for_referenceable(&referenceable)
            .into_iter()
            .flatten()
            .filter_map(|link| {
                Url::from_file_path(link.0)
                    .map(|good| Location {
                        uri: good,
                        range: link.1.data().range,
                    })
                    .ok()
            })
            .collect::<Vec<_>>(),
    )
}
