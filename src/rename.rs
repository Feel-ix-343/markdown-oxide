use std::iter;
use std::path::Path;

use tower_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    RenameFile, RenameParams, ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

use crate::vault::{Reference, Referenceable, Vault};

pub fn rename(vault: &Vault, params: &RenameParams, path: &Path) -> Option<WorkspaceEdit> {
    let position = params.text_document_position.position;
    let referenceable = vault.select_referenceable_at_position(path, position)?;

    let (referenceable_document_change, new_ref_name): (Option<DocumentChangeOperation>, String) =
        match referenceable {
            Referenceable::Heading(path, heading) => {
                let new_text = format!("{} {}", "#".repeat(heading.level.0), params.new_name); // move this obsidian syntax specific stuff to the vault

                let change_op = DocumentChangeOperation::Edit(TextDocumentEdit {
                    text_document: tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                        uri: Url::from_file_path(path).ok()?,
                        version: None,
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        range: *heading.range,
                        new_text,
                    })],
                });

                // {path name}#{new name}
                let name = format!(
                    "{}#{}",
                    path.file_stem()?.to_string_lossy().clone(),
                    params.new_name
                );

                (Some(change_op), name.to_string())
            }
            Referenceable::File(path, _file) => {
                let new_path = path.with_file_name(&params.new_name).with_extension("md");

                let change_op = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                    old_uri: Url::from_file_path(path).ok()?,
                    new_uri: Url::from_file_path(new_path.clone()).ok()?,
                    options: None,
                    annotation_id: None,
                }));

                let name = params.new_name.clone();

                (Some(change_op), name)
            }
            Referenceable::Tag(_path, _tag) => {
                let new_ref_name = params.new_name.clone();

                let _new_tag = format!("#{}", new_ref_name);

                (None, new_ref_name)
            }
            _ => return None,
        };

    let references = vault.select_references_for_referenceable(&referenceable)?;

    let references_changes = references
        .into_iter()
        .filter_map(|(path, reference)| {
            // update references

            match reference {
                // todo: move the obsidian link formatting to the vault module; it should be centralized there; no honestly this code sucks; this whole file
                Reference::WikiFileLink(data)
                    if matches!(referenceable, Referenceable::File(..)) =>
                {
                    let new_text = format!(
                        "[[{}{}]]",
                        new_ref_name,
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from(""))
                    );

                    Some(TextDocumentEdit {
                        text_document:
                            tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                                uri: Url::from_file_path(path).ok()?,
                                version: None,
                            },
                        edits: vec![OneOf::Left(TextEdit {
                            range: *data.range,
                            new_text,
                        })],
                    })
                }
                Reference::WikiHeadingLink(data, _file, infile)
                | Reference::WikiIndexedBlockLink(data, _file, infile)
                    if matches!(referenceable, Referenceable::File(..)) =>
                {
                    let new_text = format!(
                        "[[{}#{}{}]]",
                        new_ref_name,
                        infile,
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from(""))
                    );

                    Some(TextDocumentEdit {
                        text_document:
                            tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                                uri: Url::from_file_path(path).ok()?,
                                version: None,
                            },
                        edits: vec![OneOf::Left(TextEdit {
                            range: *data.range,
                            new_text,
                        })],
                    })
                }
                Reference::WikiHeadingLink(data, _file, _heading)
                    if matches!(referenceable, Referenceable::Heading(..)) =>
                {
                    let new_text = format!(
                        "[[{}{}]]",
                        new_ref_name,
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from(""))
                    );

                    Some(TextDocumentEdit {
                        text_document:
                            tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                                uri: Url::from_file_path(path).ok()?,
                                version: None,
                            },
                        edits: vec![OneOf::Left(TextEdit {
                            range: *data.range,
                            new_text,
                        })],
                    })
                }
                Reference::Tag(data) => {
                    let new_text = format!(
                        "#{}",
                        data.reference_text.replacen(
                            &*referenceable.get_refname(vault.root_dir())?,
                            &new_ref_name,
                            1
                        )
                    );

                    Some(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: Url::from_file_path(path).ok()?,
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit {
                            range: *data.range,
                            new_text,
                        })],
                    })
                }
                Reference::MDFileLink(data) if matches!(referenceable, Referenceable::File(..)) => {
                    let new_text = format!(
                        "[{}]({})",
                        data.display_text.as_deref().unwrap_or(""),
                        format_md_link_destination(&data.reference_text, &new_ref_name),
                    );

                    Some(TextDocumentEdit {
                        text_document:
                            tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                                uri: Url::from_file_path(path).ok()?,
                                version: None,
                            },
                        edits: vec![OneOf::Left(TextEdit {
                            range: *data.range,
                            new_text,
                        })],
                    })
                }

                Reference::MDHeadingLink(data, _file, infile)
                | Reference::MDIndexedBlockLink(data, _file, infile)
                    if matches!(referenceable, Referenceable::File(..)) =>
                {
                    let new_text = format!(
                        "[{}]({}#{})",
                        data.display_text.as_deref().unwrap_or(""),
                        format_md_link_destination(&data.reference_text, &new_ref_name),
                        infile,
                    );

                    Some(TextDocumentEdit {
                        text_document:
                            tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                                uri: Url::from_file_path(path).ok()?,
                                version: None,
                            },
                        edits: vec![OneOf::Left(TextEdit {
                            range: *data.range,
                            new_text,
                        })],
                    })
                }
                Reference::WikiHeadingLink(data, _file, _heading)
                    if matches!(referenceable, Referenceable::Heading(..)) =>
                {
                    let new_text = format!(
                        "[[{}{}]]",
                        new_ref_name,
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from(""))
                    );

                    Some(TextDocumentEdit {
                        text_document:
                            tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                                uri: Url::from_file_path(path).ok()?,
                                version: None,
                            },
                        edits: vec![OneOf::Left(TextEdit {
                            range: *data.range,
                            new_text,
                        })],
                    })
                }
                Reference::MDHeadingLink(_, _, _) => None,
                Reference::MDIndexedBlockLink(_, _, _) => None,
                Reference::WikiFileLink(..) => None,
                Reference::WikiHeadingLink(..) => None,
                Reference::WikiIndexedBlockLink(..) => None,
                Reference::MDFileLink(..) => None,
                Reference::Footnote(..) => None,
                Reference::LinkRef(_) => None,
            }
        })
        .map(DocumentChangeOperation::Edit);

    Some(WorkspaceEdit {
        document_changes: Some(DocumentChanges::Operations(
            references_changes
                .chain(iter::once(referenceable_document_change).flatten())
                .collect(), // order matters here
        )),
        ..Default::default()
    })
}

fn format_md_link_destination(old_ref: &str, new_name: &str) -> String {
    let (prefix, old_filename) = match old_ref.rfind('/') {
        Some(idx) => (&old_ref[..=idx], &old_ref[idx + 1..]),
        None => match old_ref.rfind('\\') {
            Some(idx) => (&old_ref[..=idx], &old_ref[idx + 1..]),
            None => ("", old_ref),
        },
    };

    let has_md = old_filename.ends_with(".md");
    let clean_new_name = if has_md && !new_name.ends_with(".md") {
        format!("{}.md", new_name)
    } else {
        new_name.to_string()
    };

    let encoded_prefix = prefix.replace(' ', "%20");
    let encoded_name = clean_new_name.replace(' ', "%20");

    format!("{}{}", encoded_prefix, encoded_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_md_link_destination() {
        // Case 1: Preserves URL-encoded directory prefix and encodes spaces in new name
        let old = "./%F0%9F%8C%80%20General/Lorem Ipsum.md";
        let formatted = format_md_link_destination(old, "Hello World");
        assert_eq!(formatted, "./%F0%9F%8C%80%20General/Hello%20World.md");

        // Case 2: Simple markdown link with extension
        let old_simple = "Lorem Ipsum.md";
        let formatted_simple = format_md_link_destination(old_simple, "Hello World");
        assert_eq!(formatted_simple, "Hello%20World.md");

        // Case 3: Subdirectory without extension
        let old_no_ext = "notes/my note";
        let formatted_no_ext = format_md_link_destination(old_no_ext, "new note");
        assert_eq!(formatted_no_ext, "notes/new%20note");

        // Case 4: Backslash paths
        let old_win = r"sub\folder\note.md";
        let formatted_win = format_md_link_destination(old_win, "another note");
        assert_eq!(formatted_win, r"sub\folder\another%20note.md");
    }

    #[test]
    fn test_rename_md_file_link_removes_pipe_and_encodes() {
        use crate::vault::{MDFile, ReferenceData};
        use tower_lsp::lsp_types::{Position, TextDocumentIdentifier, TextDocumentPositionParams};

        let root = std::env::current_dir().unwrap();
        let path = root.join("doc.md");
        let target_path = root.join("dir").join("old file.md");

        let mut md_files = std::collections::HashMap::new();
        md_files.insert(
            path.clone(),
            MDFile {
                references: vec![Reference::MDFileLink(ReferenceData {
                    reference_text: "dir/old file.md".into(),
                    range: tower_lsp::lsp_types::Range::default().into(),
                    display_text: Some("My File".into()),
                })],
                ..Default::default()
            },
        );
        md_files.insert(
            target_path.clone(),
            MDFile {
                ..Default::default()
            },
        );

        let vault = Vault {
            md_files: md_files.into(),
            ropes: std::collections::HashMap::new().into(),
            root_dir: root.clone(),
        };

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(&target_path).unwrap(),
                },
                position: Position::default(),
            },
            new_name: "new file".to_string(),
            work_done_progress_params: Default::default(),
        };

        let edit = rename(&vault, &params, &target_path).expect("Should rename");
        if let Some(DocumentChanges::Operations(ops)) = edit.document_changes {
            let text_edit = ops
                .iter()
                .find_map(|op| match op {
                    DocumentChangeOperation::Edit(edit) => Some(&edit.edits[0]),
                    _ => None,
                })
                .expect("Expected text edit");

            if let OneOf::Left(edit) = text_edit {
                assert_eq!(edit.new_text, "[My File](dir/new%20file.md)");
            } else {
                panic!("Expected OneOf::Left");
            }
        } else {
            panic!("Expected DocumentChanges::Operations");
        }
    }
}
