use std::path::Path;

use tower_lsp::lsp_types::{Position, Url, Location};

use crate::vault::Vault;

pub fn goto_definition(vault: &Vault, cursor_position: Position, path: &Path) -> Option<Vec<Location>> {
    // First, find the link that the cursor is in. Get a links for the file and match the cursor position up to one of them
    let links = vault.select_references(Some(&path))?;
    let cursors_link = links.iter().find(|&l| 
        l.1.data().range.start.line <= cursor_position.line && 
        l.1.data().range.end.line >= cursor_position.line && 
        l.1.data().range.start.character <= cursor_position.character &&
        l.1.data().range.end.character >= cursor_position.character
    )?;

    // Now we have the reference text. We need to find where this is actually referencing, or if it is referencing anything.
    // Lets get all of the referenceable nodes

    let positions = vault.select_referenceable_nodes(None);
    let referenced_linkables = positions.iter().filter(|i| i.is_reference(&vault.root_dir(), cursors_link.1, cursors_link.0));

    return Some(referenced_linkables.filter_map(|linkable| Some(Location{uri: Url::from_file_path(linkable.get_path().to_str()?).unwrap(), range: linkable.get_range()})).collect());
}
