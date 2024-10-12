use std::iter;
use std::path::Path;

use tower_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    RenameFile, RenameParams, ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

use crate::vault::{MDHeading, Reference, Referenceable, Vault};

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
                let name = format!("{}#{}", path.file_stem()?.to_string_lossy().to_owned(), params.new_name);

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
