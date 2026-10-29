use std::path::Path;

use tower_lsp::lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, Position, Range,
    SymbolKind, Url,
};

use crate::vault::{get_obsidian_ref_path, Reference, Referenceable, Vault};

pub fn referenceable_to_call_hierarchy_item(
    vault: &Vault,
    referenceable: &Referenceable,
) -> Option<CallHierarchyItem> {
    let path = referenceable.get_path();
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(vault.root_dir())
            .join(path)
    };
    let uri = Url::from_file_path(&abs_path).ok()?;
    match referenceable {
        Referenceable::File(pathbuf, _) => {
            let refname = referenceable
                .get_refname(vault.root_dir())
                .map(|r| r.to_string())
                .unwrap_or_else(|| {
                    pathbuf
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Untitled")
                        .to_string()
                });
            let detail = get_obsidian_ref_path(vault.root_dir(), pathbuf);
            let line_count = vault
                .ropes
                .get(*pathbuf)
                .map(|r| r.len_lines() as u32)
                .unwrap_or(1);
            let end_line = line_count.saturating_sub(1);
            let range = Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: end_line,
                    character: 0,
                },
            };
            let selection_range = Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            };
            Some(CallHierarchyItem {
                name: refname,
                kind: SymbolKind::FILE,
                tags: None,
                detail,
                uri,
                range,
                selection_range,
                data: None,
            })
        }
        Referenceable::Heading(pathbuf, heading) => {
            let selection_range = *heading.range;
            let headings = vault.select_headings(pathbuf);
            let start_line = heading.range.start.line;
            let end_line = headings
                .and_then(|hdgs| {
                    hdgs.iter()
                        .filter(|h| h.range.start.line > start_line && h.level <= heading.level)
                        .map(|h| h.range.start.line.saturating_sub(1))
                        .next()
                })
                .or_else(|| {
                    vault
                        .ropes
                        .get(*pathbuf)
                        .map(|r| (r.len_lines() as u32).saturating_sub(1))
                })
                .unwrap_or(heading.range.end.line);

            let range = Range {
                start: Position {
                    line: start_line,
                    character: 0,
                },
                end: Position {
                    line: end_line.max(heading.range.end.line),
                    character: 0,
                },
            };
            let detail = get_obsidian_ref_path(vault.root_dir(), pathbuf);
            Some(CallHierarchyItem {
                name: heading.heading_text.clone(),
                kind: SymbolKind::STRUCT,
                tags: None,
                detail,
                uri,
                range,
                selection_range,
                data: None,
            })
        }
        Referenceable::Tag(pathbuf, tag) => {
            let detail = get_obsidian_ref_path(vault.root_dir(), pathbuf);
            Some(CallHierarchyItem {
                name: format!("#{}", tag.tag_ref),
                kind: SymbolKind::CONSTANT,
                tags: None,
                detail,
                uri,
                range: *tag.range,
                selection_range: *tag.range,
                data: None,
            })
        }
        Referenceable::IndexedBlock(pathbuf, block) => {
            let detail = get_obsidian_ref_path(vault.root_dir(), pathbuf);
            Some(CallHierarchyItem {
                name: format!("^{}", block.index),
                kind: SymbolKind::KEY,
                tags: None,
                detail,
                uri,
                range: *block.range,
                selection_range: *block.range,
                data: None,
            })
        }
        Referenceable::Footnote(pathbuf, footnote) => {
            let detail = get_obsidian_ref_path(vault.root_dir(), pathbuf);
            Some(CallHierarchyItem {
                name: format!("[^{}]", footnote.index),
                kind: SymbolKind::KEY,
                tags: None,
                detail,
                uri,
                range: *footnote.range,
                selection_range: *footnote.range,
                data: None,
            })
        }
        Referenceable::LinkRefDef(pathbuf, refdef) => {
            let detail = get_obsidian_ref_path(vault.root_dir(), pathbuf);
            Some(CallHierarchyItem {
                name: refdef.link_ref_name.clone(),
                kind: SymbolKind::KEY,
                tags: None,
                detail,
                uri,
                range: *refdef.range,
                selection_range: *refdef.range,
                data: None,
            })
        }
        _ => None,
    }
}

pub fn prepare_call_hierarchy(
    vault: &Vault,
    path: &Path,
    position: Position,
) -> Option<Vec<CallHierarchyItem>> {
    // 1. If cursor is on a reference (link), try to resolve to its targets
    if let Some(reference) = vault.select_reference_at_position(path, position) {
        let targets = vault.select_referenceables_for_reference(reference, path);
        let items: Vec<CallHierarchyItem> = targets
            .iter()
            .filter_map(|t| referenceable_to_call_hierarchy_item(vault, t))
            .collect();
        if !items.is_empty() {
            return Some(items);
        }
    }

    // 2. Otherwise get the referenceable at the cursor (Heading, Tag, or File)
    let referenceable = vault.select_referenceable_at_position(path, position)?;
    let item = referenceable_to_call_hierarchy_item(vault, &referenceable)?;
    Some(vec![item])
}

pub fn incoming_calls(
    vault: &Vault,
    item: &CallHierarchyItem,
) -> Option<Vec<CallHierarchyIncomingCall>> {
    let path = item.uri.to_file_path().ok()?;

    let target_ref = if item.kind == SymbolKind::FILE {
        vault
            .md_files
            .get_key_value(&path)
            .map(|(p, f)| Referenceable::File(p, f))
    } else {
        vault.select_referenceable_at_position(&path, item.selection_range.start)
    }?;

    let references = vault.select_references_for_referenceable(&target_ref)?;
    if references.is_empty() {
        return Some(vec![]);
    }

    let mut grouped_calls: Vec<(CallHierarchyItem, Vec<Range>)> = Vec::new();

    for (caller_path, reference) in references {
        let ref_range = *reference.data().range;
        let caller_heading = vault.select_headings(caller_path).and_then(|hdgs| {
            hdgs.iter()
                .filter(|h| h.range.start.line <= ref_range.start.line)
                .max_by_key(|h| h.range.start.line)
        });

        let (file_pathbuf, file_md) = vault.md_files.get_key_value(caller_path)?;

        let caller_item = match caller_heading {
            Some(h) => referenceable_to_call_hierarchy_item(
                vault,
                &Referenceable::Heading(file_pathbuf, h),
            ),
            None => referenceable_to_call_hierarchy_item(
                vault,
                &Referenceable::File(file_pathbuf, file_md),
            ),
        };

        if let Some(c_item) = caller_item {
            if let Some(existing) = grouped_calls.iter_mut().find(|(existing_item, _)| {
                existing_item.uri == c_item.uri
                    && existing_item.selection_range == c_item.selection_range
            }) {
                if !existing.1.contains(&ref_range) {
                    existing.1.push(ref_range);
                }
            } else {
                grouped_calls.push((c_item, vec![ref_range]));
            }
        }
    }

    Some(
        grouped_calls
            .into_iter()
            .map(|(from, from_ranges)| CallHierarchyIncomingCall { from, from_ranges })
            .collect(),
    )
}

pub fn outgoing_calls(
    vault: &Vault,
    item: &CallHierarchyItem,
) -> Option<Vec<CallHierarchyOutgoingCall>> {
    let path = item.uri.to_file_path().ok()?;

    let all_refs = vault.select_references(Some(&path))?;

    let filtered_refs: Vec<&Reference> = if item.kind == SymbolKind::FILE {
        all_refs.into_iter().map(|(_, r)| r).collect()
    } else {
        all_refs
            .into_iter()
            .map(|(_, r)| r)
            .filter(|r| {
                let r_range = r.data().range;
                r_range.start.line >= item.range.start.line
                    && r_range.end.line <= item.range.end.line
            })
            .collect()
    };

    if filtered_refs.is_empty() {
        return Some(vec![]);
    }

    let mut grouped_calls: Vec<(CallHierarchyItem, Vec<Range>)> = Vec::new();

    for reference in filtered_refs {
        let ref_range = *reference.data().range;
        let targets = vault.select_referenceables_for_reference(reference, &path);
        for target in targets {
            if let Some(target_item) = referenceable_to_call_hierarchy_item(vault, &target) {
                if let Some(existing) = grouped_calls.iter_mut().find(|(existing_item, _)| {
                    existing_item.uri == target_item.uri
                        && existing_item.selection_range == target_item.selection_range
                }) {
                    if !existing.1.contains(&ref_range) {
                        existing.1.push(ref_range);
                    }
                } else {
                    grouped_calls.push((target_item, vec![ref_range]));
                }
            }
        }
    }

    Some(
        grouped_calls
            .into_iter()
            .map(|(to, from_ranges)| CallHierarchyOutgoingCall { to, from_ranges })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use tower_lsp::lsp_types::{ClientCapabilities, Position, SymbolKind};

    use super::*;
    use crate::config::Settings;

    fn test_settings() -> Settings {
        Settings::new(Path::new("."), &ClientCapabilities::default()).unwrap()
    }

    fn setup_test_vault() -> (Vault, PathBuf, PathBuf) {
        let root = std::env::current_dir().unwrap().join("test_vault");
        let path_a = root.join("NoteA.md");
        let path_b = root.join("NoteB.md");

        let text_a = "# Note A\nThis links to [[NoteB]] and also to [[NoteB#SubSection]].\n";
        let text_b = "# Note B\nSome intro text.\n## SubSection\nDetails here.\n";

        let mut vault = Vault {
            md_files: HashMap::new().into(),
            ropes: HashMap::new().into(),
            root_dir: root.clone(),
        };

        Vault::update_vault(&test_settings(), &mut vault, (&path_a, text_a));
        Vault::update_vault(&test_settings(), &mut vault, (&path_b, text_b));

        (vault, path_a, path_b)
    }

    #[test]
    fn test_prepare_call_hierarchy_on_file() {
        let (vault, path_a, _) = setup_test_vault();
        let items = prepare_call_hierarchy(
            &vault,
            &path_a,
            Position {
                line: 1,
                character: 0,
            },
        )
        .expect("prepare_call_hierarchy on file text should succeed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::FILE);
        assert_eq!(items[0].name, "NoteA");
    }

    #[test]
    fn test_prepare_call_hierarchy_on_heading() {
        let (vault, _, path_b) = setup_test_vault();
        // Line 2 in NoteB is "## SubSection"
        let items = prepare_call_hierarchy(
            &vault,
            &path_b,
            Position {
                line: 2,
                character: 5,
            },
        )
        .expect("prepare_call_hierarchy on heading should succeed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::STRUCT);
        assert_eq!(items[0].name, "SubSection");
    }

    #[test]
    fn test_prepare_call_hierarchy_on_link() {
        let (vault, path_a, _) = setup_test_vault();
        // Line 1 contains "[[NoteB]]" around character 16
        let items = prepare_call_hierarchy(
            &vault,
            &path_a,
            Position {
                line: 1,
                character: 16,
            },
        )
        .expect("prepare_call_hierarchy on link should resolve target");
        assert!(!items.is_empty());
        assert_eq!(items[0].name, "NoteB");
        assert_eq!(items[0].kind, SymbolKind::FILE);
    }

    #[test]
    fn test_incoming_calls() {
        let (vault, _, path_b) = setup_test_vault();
        // Prepare NoteB
        let items = prepare_call_hierarchy(
            &vault,
            &path_b,
            Position {
                line: 1,
                character: 0,
            },
        )
        .expect("prepare NoteB");
        let incoming = incoming_calls(&vault, &items[0]).expect("incoming calls to NoteB");
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].from.name, "Note A");
        assert_eq!(incoming[0].from_ranges.len(), 2);
    }

    #[test]
    fn test_outgoing_calls() {
        let (vault, path_a, _) = setup_test_vault();
        // Prepare NoteA
        let items = prepare_call_hierarchy(
            &vault,
            &path_a,
            Position {
                line: 0,
                character: 0,
            },
        )
        .expect("prepare NoteA");
        let outgoing = outgoing_calls(&vault, &items[0]).expect("outgoing calls from NoteA");
        // NoteA links to NoteB and NoteB#SubSection
        assert_eq!(outgoing.len(), 2);
        let names: Vec<String> = outgoing.iter().map(|o| o.to.name.clone()).collect();
        assert!(names.contains(&"NoteB".to_string()));
        assert!(names.contains(&"SubSection".to_string()));
    }
}
