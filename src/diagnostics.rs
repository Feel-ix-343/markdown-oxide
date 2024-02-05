use std::path::PathBuf;

use rayon::prelude::*;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Url};

use crate::vault::{self, Vault, Referenceable};

pub fn diagnostics(vault: &Vault, (path, _uri): (&PathBuf, &Url)) -> Option<Vec<Diagnostic>> {
    // Diagnostics
    // get all links for changed file
    let referenceables = vault.select_referenceable_nodes(None);
    let pathreferences = vault.select_references(Some(path))?;
    let allreferences = vault.select_references(None)?;
    let unresolved = pathreferences.par_iter().filter(|(path, reference)| {
        let matched_option = referenceables
            .iter()
            .find(|referenceable| reference.references(vault.root_dir(), path, referenceable));

        matched_option.is_some_and(|matched| {
            return matches!(matched, Referenceable::UnresovledIndexedBlock(..) | Referenceable::UnresovledFile(..) | Referenceable::UnresolvedHeading(..))
        })
    });

    let diags: Vec<Diagnostic> = unresolved
        .map(|(path, reference)| Diagnostic {
            range: *reference.data().range,
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
                _ => "Unresolved Reference".to_string(),
            },
            source: Some("Obsidian LS".into()),
            severity: Some(DiagnosticSeverity::INFORMATION),
            ..Default::default()
        })
        .collect();

    return Some(diags);
}
