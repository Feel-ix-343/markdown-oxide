use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Url};

use crate::{
    config::Settings,
    vault::{self, Reference, Referenceable, Vault},
};

pub fn path_unresolved_references<'a>(
    vault: &'a Vault,
    path: &'a Path,
) -> Option<Vec<(&'a Path, &'a Reference)>> {
    let referenceables = vault.select_referenceable_nodes(None);
    let pathreferences = vault.select_references(Some(path))?;

    let unresolved = pathreferences
        .into_par_iter()
        .filter(|(path, reference)| {
            let matched_option = referenceables
                .iter()
                .find(|referenceable| reference.references(vault.root_dir(), path, referenceable));

            matched_option.is_some_and(|matched| {
                matches!(
                    matched,
                    Referenceable::UnresovledIndexedBlock(..)
                        | Referenceable::UnresovledFile(..)
                        | Referenceable::UnresolvedHeading(..)
                )
            })
        })
        .collect::<Vec<_>>();

    Some(unresolved)
}

pub fn diagnostics(
    vault: &Vault,
    settings: &Settings,
    (path, _uri): (&PathBuf, &Url),
) -> Option<Vec<Diagnostic>> {
    if !settings.unresolved_diagnostics {
        return None;
    }

    let unresolved = path_unresolved_references(vault, path)?;

    let allreferences = vault.select_references(None)?;

    let diags: Vec<Diagnostic> = unresolved
        .into_par_iter()
        .map(|(path, reference)| Diagnostic {
            range: *reference.data().range,
            message: match allreferences
                .iter()
                .filter(|(other_path, otherreference)| {
                    otherreference.matches_type(reference)
                        && (!matches!(reference, vault::Reference::Footnote(_))
                            || **other_path == *path)
                        && otherreference.data().reference_text == reference.data().reference_text
                })
                .count()
            {
                num if num > 1 => format!("Unresolved Reference used {} times", num),
                _ => "Unresolved Reference".to_string(),
            },
            source: Some("markdown-oxide".into()),
            severity: Some(DiagnosticSeverity::INFORMATION),
            ..Default::default()
        })
        .collect();

    Some(diags)
}
