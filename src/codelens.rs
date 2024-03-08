use std::path::Path;

use itertools::Itertools;
use tower_lsp::lsp_types::{CodeLens, CodeLensParams, Command, Location, Position, Url};

use crate::vault::{Vault, Referenceable};

use serde::Serialize;

#[derive(Serialize)]
struct FindReferencesData {
    uri: Url,
    position: Position,
    locations: Vec<Location>,
}

pub fn code_lens(vault: &Vault, path: &Path, _params: &CodeLensParams) -> Option<Vec<CodeLens>> {
    let referenceables = vault.select_referenceable_nodes(Some(path));
    let data = referenceables
        .into_iter()
        .filter_map(|referenceable| {
            let references = vault
                .select_references_for_referenceable(&referenceable)?
                .into_iter()
                .collect_vec();

            Some((referenceable, references))
        })
        .collect_vec();

    let lens = data
        .into_iter()
        .filter(|(_, references)| !references.is_empty())
        .filter_map(|(referenceable, references)| {
            let title = match references.len() {
                1 => "1 reference".into(),
                n => format!("{} references", n),
            };

            let locations = references
                .into_iter()
                .filter_map(|(path, reference)| {
                    Some(Location {
                        uri: Url::from_file_path(path).ok()?,
                        range: *reference.data().range,
                    })
                })
                .collect_vec();

            let range = match referenceable {
                    Referenceable::File(..) => tower_lsp::lsp_types::Range { 
                        start: Position{
                            line: 0,
                            character: 0
                        }, end: Position {
                            line: 0,
                            character: 1
                        }
                    },
                    _ => *referenceable.get_range()?
                };

            Some(CodeLens {
                range,
                command: Some(Command {
                    title,
                    command: "moxide.findReferences".into(),
                    arguments: Some(vec![serde_json::to_value(FindReferencesData {
                        uri: Url::from_file_path(path).ok()?,
                        position: range.start,
                        locations,
                    })
                    .ok()?]),
                }),
                data: None,
            })
        })
        .collect_vec();

    Some(lens)
}
