use std::path::Path;

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::vault::Vault;

pub fn goto_definition(
    vault: &Vault,
    cursor_position: Position,
    path: &Path,
) -> Option<Vec<Location>> {
    // First, find the link that the cursor is in. Get a links for the file and match the cursor position up to one of them
    let links = vault.select_references(Some(&path))?;
    let (path, reference) = links.iter().find(|&l| {
        l.1.data().range.start.line <= cursor_position.line
            && l.1.data().range.end.line >= cursor_position.line
            && l.1.data().range.start.character <= cursor_position.character
            && l.1.data().range.end.character >= cursor_position.character
    })?;

    // Now we have the reference text. We need to find where this is actually referencing, or if it is referencing anything.
    // Lets get all of the referenceable nodes

    let positions = vault.select_referenceable_nodes(None);
    let referenced_referenceables = positions
        .iter()
        .filter(|i| reference.references(&vault.root_dir(), path, i));

    return Some(
        referenced_referenceables
            .filter_map(|linkable| {
                Some(Location {
                    uri: Url::from_file_path(linkable.get_path().to_str()?).unwrap(),
                    range: linkable.get_range(),
                })
            })
            .collect(),
    );
}
