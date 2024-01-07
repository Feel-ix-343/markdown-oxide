use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{Position, Location, Url};

use crate::vault::{Vault, Referenceable};

pub fn references(vault: &Vault, cursor_position: Position, path: &Path) -> Option<Vec<Location>> {
    // First we need to get the referenceable node under the cursor
    let path = path.to_path_buf();
    let linkable_nodes = vault.select_linkable_nodes_for_path(&path)?;
    let linkable = linkable_nodes
        .iter()
        .find(|&l| 
            l.get_range().start.line <= cursor_position.line && 
            l.get_range().end.line >= cursor_position.line && 
            l.get_range().start.character <= cursor_position.character &&
            l.get_range().end.character >= cursor_position.character
        )?;


    let references = vault.select_references(None)?;
    let locations = |referenceable: &Referenceable| references.iter()
        .filter(|&r| referenceable.is_reference(&vault.root_dir(), &r.1, r.0))
        .map(|link| Url::from_file_path(link.0).map(|good| Location {uri: good, range: link.1.range}))
        .flat_map(|l| match l.is_ok() {
            true => Some(l),
            false => None
        })
        .flatten()
        .collect_vec();

    return match linkable {
        Referenceable::File(_, _) => {
            return Some(linkable_nodes.iter()
                .filter(|&referenceable| !matches!(referenceable, &Referenceable::Tag(_, _)) && !matches!(referenceable, &Referenceable::Footnote(_, _)))
                .map(|referenceable| locations(referenceable))
                .flatten()
                .collect())
        }
        _ => Some(locations(linkable))
    }
}
