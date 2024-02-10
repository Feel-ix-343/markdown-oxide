use std::path::Path;

use itertools::Itertools;
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{CodeLens, CodeLensParams, Command, Location, Position, Url},
};

use crate::vault::Vault;

use serde::Serialize;

#[derive(Serialize)]
struct FindReferencesData {
    uri: Url,
    position: Position,
    locations: Vec<Location>,
}

pub fn code_lens(vault: &Vault, path: &Path, params: &CodeLensParams) -> Option<Vec<CodeLens>> {
    let referenceables = vault.select_referenceable_nodes(Some(&path));
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
            let title = format!("{} references", references.len());

            let locations = references
                .into_iter()
                .filter_map(|(path, reference)| {
                    Some(Location {
                        uri: Url::from_file_path(path).ok()?,
                        range: *reference.data().range.clone(),
                    })
                })
                .collect_vec();

            Some(CodeLens {
                range: *referenceable.get_range()?,
                command: Some(Command {
                    title,
                    command: "moxide.findReferences".into(),
                    arguments: Some(vec![serde_json::to_value(FindReferencesData {
                        uri: Url::from_file_path(path).ok()?,
                        position: referenceable.get_range()?.start,
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
