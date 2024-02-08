use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{Location, Position, Url};

use crate::vault::{Vault, Reference};

pub fn references(vault: &Vault, cursor_position: Position, path: &Path) -> Option<Vec<Location>> {

    let references: Vec<(&Path, &Reference)> = vault.select_referenceable_at_position(path, cursor_position)
        .and_then(|referenceable| {

            vault.select_references_for_referenceable(&referenceable)

        })
        .or(
            vault.select_reference_at_position(path, cursor_position)
                .and_then(|reference| {

                    vault.select_references(None)

                        .map(|references| {
                            references
                                .into_iter()
                                .filter(|(p, r)| reference.matches_reference(vault, path, (r, p)))
                                .collect_vec()
                        })


                })
        )?;


    Some( references
            .into_iter()
            .filter_map(|link| {
                Url::from_file_path(link.0)
                    .map(|good| Location {
                        uri: good,
                        range: *link.1.data().range // TODO: Why can't I use .into() here?
                    })
                    .ok()
            })
            .collect::<Vec<_>>(),
    )
}
