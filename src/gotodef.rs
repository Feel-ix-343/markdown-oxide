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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use tower_lsp::lsp_types::{ClientCapabilities, Position, Url};

    use crate::{config::Settings, vault::Vault};

    use super::goto_definition;

    fn temp_vault_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("markdown-oxide-gotodef-{nanos}"))
    }

    #[test]
    fn wiki_link_to_filename_with_spaces_resolves() {
        let root = temp_vault_dir();
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.md");
        let target = root.join("git command.md");
        fs::write(&source, "See [[git command]]\n").unwrap();
        fs::write(&target, "# Git Command\n").unwrap();

        let settings = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        let vault = Vault::construct_vault(&settings, &root).unwrap();
        let definitions = goto_definition(
            &vault,
            Position {
                line: 0,
                character: 8,
            },
            &source,
        )
        .unwrap();

        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].uri, Url::from_file_path(&target).unwrap());

        fs::remove_dir_all(root).unwrap();
    }
}
