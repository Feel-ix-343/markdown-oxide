use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{Position, Location, Url};

use crate::vault::{Vault, Referenceable};

pub fn references(vault: &Vault, cursor_position: Position, path: &Path) -> Option<Vec<Location>> {
    // First we need to get the referenceable node under the cursor
    let pathbuf = path.to_path_buf();
    let linkable_nodes = vault.select_referenceable_nodes(Some(&pathbuf));
    let linkable = vault.select_referenceable_at_position(path, cursor_position)?;


    let locations = |referenceable: Referenceable| vault.select_references_for_referenceable(&referenceable)
        .into_iter()
        .flatten()
        .map(|link| Url::from_file_path(link.0).map(|good| Location {uri: good, range: link.1.data().range}))
        .flat_map(|l| match l.is_ok() {
            true => Some(l),
            false => None
        })
        .flatten()
        .collect_vec();

    return match linkable {
        Referenceable::File(_, _) => {
            return Some(linkable_nodes.into_iter()
                .filter(|referenceable| !matches!(referenceable, Referenceable::Tag(_, _)) && !matches!(referenceable, Referenceable::Footnote(_, _)))
                .map(|referenceable| locations(referenceable))
                .flatten()
                .collect())
        }
        _ => Some(locations(linkable))
    }
}
