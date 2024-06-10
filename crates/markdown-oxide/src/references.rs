use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{Location, Position, Url};

use vault::{Referenceable, Vault};

pub fn references(vault: &Vault, cursor_position: Position, path: &Path) -> Option<Vec<Location>> {
    let references = match (
        vault.select_referenceable_at_position(path, cursor_position),
        vault.select_reference_at_position(path, cursor_position),
    ) {
        (Some(referenceable @ Referenceable::Tag(..)), Some(_)) | (Some(referenceable), None) => {
            vault.select_references_for_referenceable(&referenceable)
        }
        (_, Some(reference)) => {
            let referenceables = vault.select_referenceables_for_reference(reference, path);
            let references = referenceables
                .into_iter()
                .filter_map(|referenceable| {
                    vault.select_references_for_referenceable(&referenceable)
                }) // drop the Nones on the options
                .flatten()
                .collect_vec();

            Some(references)
        }
        (None, None) => None,
    }?;

    Some(
        references
            .into_iter()
            .filter_map(|link| {
                Url::from_file_path(link.0)
                    .map(|good| Location {
                        uri: good,
                        range: *link.1.data().range, // TODO: Why can't I use .into() here?
                    })
                    .ok()
            })
            .collect::<Vec<_>>(),
    )
}
