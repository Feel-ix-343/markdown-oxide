use std::path::Path;

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::vault::{Referenceable, Vault};

pub fn goto_definition(
    vault: &Vault,
    cursor_position: Position,
    path: &Path,
) -> Option<Vec<Location>> {
    // First, find the link that the cursor is in. Get a links for the file and match the cursor position up to one of them
    let reference = vault.select_reference_at_position(path, cursor_position)?;
    // Now we have the reference text. We need to find where this is actually referencing, or if it is referencing anything.
    // Lets get all of the referenceable nodes

    let referenceables = vault.select_referenceables_for_reference(reference, path);

    Some(
        referenceables
            .into_iter()
            .filter_map(|linkable| {
                let range = match linkable {
                    Referenceable::File(..) => tower_lsp::lsp_types::Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 1,
                        },
                    },
                    _ => *linkable.get_range()?,
                };

                Some(Location {
                    uri: Url::from_file_path(linkable.get_path().to_str()?).unwrap(),
                    range,
                })
            })
            .collect(),
    )
}
