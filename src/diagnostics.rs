use std::path::PathBuf;

use rayon::prelude::*;
use tower_lsp::{
    lsp_types::{Diagnostic, DiagnosticSeverity, Url},
    Client,
};

use crate::vault::{self, Vault};

pub async fn diagnostics(vault: &Vault, (path, uri, _): (&PathBuf, &Url, &str), client: &Client) {
    // Diagnostics
    // get all links for changed file
    let referenceables = vault.select_referenceable_nodes(None);
    let Some(pathreferences) = vault.select_references(Some(&path)) else {
        return;
    };
    let Some(allreferences) = vault.select_references(None) else {
        return;
    };
    let unresolved = pathreferences.par_iter().filter(|(path, reference)| {
        !referenceables.iter().any(|referenceable| {
            referenceable.matches_reference(&vault.root_dir(), reference, path)
        })
    });

    let diags: Vec<Diagnostic> = unresolved
        .map(|(path, reference)| Diagnostic {
            range: reference.data().range,
            message: match allreferences
                .iter()
                .filter(|(other_path, otherreference)| {
                    otherreference.matches_type(reference)
                        && (!matches!(reference, vault::Reference::Footnote(_))
                            || *other_path == *path)
                        && otherreference.data().reference_text == reference.data().reference_text
                })
                .count()
            {
                num if num > 1 => format!("Unresolved Reference used {} times", num),
                _ => format!("Unresolved Reference"),
            },
            source: Some("Obsidian LS".into()),
            severity: Some(DiagnosticSeverity::INFORMATION),
            ..Default::default()
        })
        .collect();

    client.publish_diagnostics(uri.clone(), diags, None).await;
}
