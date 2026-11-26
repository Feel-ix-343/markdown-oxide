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

    let mut diags: Vec<Diagnostic> = unresolved
        .into_par_iter()
        .map(|(path, reference)| Diagnostic {
            range: *reference.data().range,
            message: unresolved_message(
                vault,
                path,
                reference,
                match allreferences
                    .iter()
                    .filter(|(other_path, otherreference)| {
                        otherreference.matches_type(reference)
                            && (!matches!(reference, vault::Reference::Footnote(_))
                                || **other_path == *path)
                            && otherreference.data().reference_text
                                == reference.data().reference_text
                    })
                    .count()
                {
                    num if num > 1 => format!("Unresolved Reference used {} times", num),
                    _ => "Unresolved Reference".to_string(),
                },
            ),
            source: Some("markdown-oxide".into()),
            severity: Some(DiagnosticSeverity::INFORMATION),
            ..Default::default()
        })
        .collect();

    if settings.ambiguous_diagnostics {
        diags.extend(ambiguous_diagnostics(vault, path).unwrap_or_default());
    }

    Some(diags)
}

/// Keep the upstream `Unresolved Reference…` prefix; append why, when it is knowable.
fn unresolved_message(vault: &Vault, path: &Path, reference: &Reference, base: String) -> String {
    let text = &reference.data().reference_text;
    let file_part = text.split_once('#').map_or(text.as_str(), |(file, _)| file);
    if is_placeholder(file_part) {
        return format!("{base} (placeholder or pattern: write it as code, or link a real note)");
    }
    if let Some(outside) = existing_unindexed_target(vault, path, reference, file_part) {
        return format!("{base} (target exists outside the indexed vault: {outside})");
    }
    base
}

fn is_placeholder(file_part: &str) -> bool {
    file_part.contains('*')
        || file_part.contains("...")
        || file_part.contains('<')
        || file_part
            .split(['-', '/', '_', ' '])
            .any(|part| part == "XX")
}

/// A file on disk under the vault root (e.g. in an excluded folder) that the link names.
fn existing_unindexed_target(
    vault: &Vault,
    path: &Path,
    reference: &Reference,
    file_part: &str,
) -> Option<String> {
    let root = vault.root_dir();
    let candidates = reference
        .relative_ref_path(root, path, file_part)
        .into_iter()
        .chain(std::iter::once(
            file_part
                .trim_start_matches("./")
                .trim_start_matches('/')
                .to_string(),
        ));
    for candidate in candidates {
        if candidate.is_empty() {
            continue;
        }
        for name in [candidate.clone(), format!("{candidate}.md")] {
            let full = root.join(&name);
            let inside = full
                .canonicalize()
                .ok()
                .zip(root.canonicalize().ok())
                .is_some_and(|(full, root)| full.starts_with(root));
            if inside && full.is_file() && !vault.md_files.contains_key(&full) {
                return Some(name);
            }
        }
    }
    None
}

/// Links whose file part matches more than one indexed note.
fn ambiguous_diagnostics(vault: &Vault, path: &Path) -> Option<Vec<Diagnostic>> {
    let files: Vec<Referenceable> = vault
        .md_files
        .iter()
        .map(|(file_path, file)| Referenceable::File(file_path, file))
        .collect();
    let references = vault.select_references(Some(path))?;
    Some(
        references
            .into_par_iter()
            .filter(|(_, reference)| {
                !matches!(
                    reference,
                    Reference::Tag(_) | Reference::Footnote(_) | Reference::LinkRef(_)
                )
            })
            .filter_map(|(ref_path, reference)| {
                // A link whose relative target exists resolves to exactly that note.
                let text = &reference.data().reference_text;
                let file_part = text.split_once('#').map_or(text.as_str(), |(file, _)| file);
                if let Some(relative) =
                    reference.relative_ref_path(vault.root_dir(), ref_path, file_part)
                {
                    if vault
                        .md_files
                        .contains_key(&vault.root_dir().join(format!("{relative}.md")))
                    {
                        return None;
                    }
                }
                let mut matches: Vec<String> = files
                    .iter()
                    .filter(|file| file.matches_reference(vault.root_dir(), reference, ref_path))
                    .filter_map(|file| {
                        vault::get_obsidian_ref_path(vault.root_dir(), file.get_path())
                    })
                    .collect();
                if matches.len() < 2 {
                    return None;
                }
                matches.sort();
                Some(Diagnostic {
                    range: *reference.data().range,
                    message: format!(
                        "Ambiguous Reference: matches {} notes ({})",
                        matches.len(),
                        matches.join(", ")
                    ),
                    source: Some("markdown-oxide".into()),
                    severity: Some(DiagnosticSeverity::WARNING),
                    ..Default::default()
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod kb_customization_tests {
    use std::path::Path;

    use tower_lsp::lsp_types::{ClientCapabilities, DiagnosticSeverity, Url};

    use crate::config::Settings;
    use crate::vault::Vault;

    fn vault(files: &[(&str, &str)]) -> (std::path::PathBuf, Vault) {
        let root = std::env::temp_dir().join(format!(
            "moxide-kb-diag-{}-{}",
            std::process::id(),
            files.len()
        ));
        let _ = std::fs::remove_dir_all(&root);
        for (rel, text) in files {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let settings = Settings::new(Path::new("."), &ClientCapabilities::default()).unwrap();
        let vault = Vault::construct_vault(&settings, &root).unwrap();
        (root, vault)
    }

    fn messages(
        root: &Path,
        vault: &Vault,
        rel: &str,
        ambiguous: bool,
    ) -> Vec<(String, DiagnosticSeverity)> {
        let mut settings = Settings::new(Path::new("."), &ClientCapabilities::default()).unwrap();
        settings.unresolved_diagnostics = true;
        settings.ambiguous_diagnostics = ambiguous;
        let path = root.join(rel);
        let uri = Url::from_file_path(&path).unwrap();
        let mut out: Vec<_> = super::diagnostics(vault, &settings, (&path, &uri))
            .unwrap()
            .into_iter()
            .map(|d| (d.message, d.severity.unwrap()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    #[test]
    fn explains_placeholders_and_unindexed_targets_and_flags_ambiguity() {
        let (root, vault) = vault(&[
            ("a/note.md", "# A\n"),
            ("b/note.md", "# B\n"),
            (
                "books/ref.md",
                "# outside, but excluded by the test below\n",
            ),
            (
                "a/links.md",
                concat!(
                    "[[ses-agent-*|glob]] [[2026-06-XX-plan|date]] [[gone|missing]]\n",
                    "[[note|ambiguous]] [same](note.md) [[a/note|exact]]\n",
                ),
            ),
        ]);
        // Index again with books/ excluded, so books/ref.md is on disk but not in the vault.
        std::fs::write(root.join("a/boundary.md"), "[[../books/ref|book]]\n").unwrap();
        let mut settings = Settings::new(Path::new("."), &ClientCapabilities::default()).unwrap();
        settings.excluded_folders = vec!["books".into()];
        let vault_excluded = Vault::construct_vault(&settings, &root).unwrap();

        let unresolved_only = messages(&root, &vault, "a/links.md", false);
        assert_eq!(
            unresolved_only,
            vec![
                ("Unresolved Reference".to_string(), DiagnosticSeverity::INFORMATION),
                ("Unresolved Reference (placeholder or pattern: write it as code, or link a real note)".to_string(), DiagnosticSeverity::INFORMATION),
                ("Unresolved Reference (placeholder or pattern: write it as code, or link a real note)".to_string(), DiagnosticSeverity::INFORMATION),
            ]
        );

        let with_ambiguity = messages(&root, &vault, "a/links.md", true);
        let ambiguous: Vec<_> = with_ambiguity
            .iter()
            .filter(|(m, _)| m.starts_with("Ambiguous"))
            .collect();
        assert_eq!(
            ambiguous.len(),
            1,
            "only [[note]]; the relative [same](note.md) and [[a/note]] are exact"
        );
        assert_eq!(
            ambiguous[0].0,
            "Ambiguous Reference: matches 2 notes (a/note, b/note)"
        );
        assert_eq!(ambiguous[0].1, DiagnosticSeverity::WARNING);

        assert_eq!(
            messages(&root, &vault_excluded, "a/boundary.md", false),
            vec![(
                "Unresolved Reference (target exists outside the indexed vault: books/ref.md)"
                    .to_string(),
                DiagnosticSeverity::INFORMATION
            )]
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
