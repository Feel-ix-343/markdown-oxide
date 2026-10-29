use std::iter;
use std::path::Path;

use tower_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    RenameFile, RenameParams, ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

use crate::vault::{ensure_md_extension, Reference, Referenceable, Vault};

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
                let file_name = ensure_md_extension(&params.new_name);
                let new_path = path.with_file_name(&file_name);

                let change_op = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                    old_uri: Url::from_file_path(path).ok()?,
                    new_uri: Url::from_file_path(new_path.clone()).ok()?,
                    options: None,
                    annotation_id: None,
                }));

                let name = params
                    .new_name
                    .strip_suffix(".md")
                    .unwrap_or(&params.new_name)
                    .to_string();

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
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from("")),
                        new_ref_name,
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
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from("")),
                        new_ref_name,
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
                        "[{}]({})",
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from("")),
                        new_ref_name,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use std::fs;
    use tower_lsp::lsp_types::{ClientCapabilities, Position, RenameParams, TextDocumentIdentifier, TextDocumentPositionParams};

    #[test]
    fn test_rename_file_preserves_dots_in_name() {
        let unique_dir = format!("test_moxide_rename_dot_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let root = std::env::temp_dir().join(unique_dir);
        fs::create_dir_all(&root).unwrap();
        let file_path = root.join("OldNote.md");
        let ref_file = root.join("Referrer.md");
        fs::write(&file_path, "Plain note content\n").unwrap();
        fs::write(&ref_file, "Link to [[OldNote]]\n").unwrap();

        let settings = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        let vault = Vault::construct_vault(&settings, &root).unwrap();

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(&file_path).unwrap(),
                },
                position: Position { line: 0, character: 0 },
            },
            new_name: "TECH.51.05-New title".to_string(),
            work_done_progress_params: Default::default(),
        };

        let edit = rename(&vault, &params, &file_path).unwrap();
        let ops = match edit.document_changes.unwrap() {
            DocumentChanges::Operations(ops) => ops,
            _ => panic!("Expected DocumentChanges::Operations"),
        };

        let rename_op = ops.iter().find_map(|op| match op {
            DocumentChangeOperation::Op(ResourceOp::Rename(r)) => Some(r),
            _ => None,
        }).expect("Rename operation not found");
        assert_eq!(rename_op.new_uri.to_file_path().unwrap().file_name().unwrap(), "TECH.51.05-New title.md");

        let edit_op = ops.iter().find_map(|op| match op {
            DocumentChangeOperation::Edit(e) => Some(e),
            _ => None,
        }).expect("Edit operation not found");
        match &edit_op.edits[0] {
            OneOf::Left(te) => assert_eq!(te.new_text, "[[TECH.51.05-New title]]"),
            _ => panic!("Expected TextEdit"),
        }

        let _ = fs::remove_dir_all(&root);
    }
}
