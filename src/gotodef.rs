use std::path::Path;

use tower_lsp::lsp_types::{Position, Url, Location};

use crate::vault::Vault;

pub fn goto_definition(vault: &Vault, cursor_position: Position, path: &Path) -> Option<Location> {
    // First, find the link that the cursor is in. Get a links for the file and match the cursor position up to one of them
    let links = vault.select_links(Some(&path))?;
    let cursors_link = links.iter().find(|&l| 
        l.1.range.start.line <= cursor_position.line && 
        l.1.range.end.line >= cursor_position.line && 
        l.1.range.start.character <= cursor_position.character &&
        l.1.range.end.character >= cursor_position.character
    )?;
    let reference_text = &cursors_link.1.reference_text;

    // Now we have the reference text. We need to find where this is actually referencing, or if it is referencing anything.
    // Lets get all of the linkable nodes

    let positions = vault.select_linkable_nodes();
    let referenced_linkable = positions.iter().find(|i| i.get_refname(&vault.root_dir()).as_ref() == Some(&reference_text))?;
    let path_as_str = referenced_linkable.get_path().to_str()?;

    return Some(Location { uri: Url::from_file_path(path_as_str).unwrap(), range: referenced_linkable.get_range() })
}
