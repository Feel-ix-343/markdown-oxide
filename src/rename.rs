use std::iter;
use std::path::Path;
use std::str::FromStr as _;

use itertools::Itertools;
use pathdiff::diff_paths;
use tower_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    RenameFile, RenameFilesParams, RenameParams, ResourceOp, TextDocumentEdit, TextEdit, Url,
    WorkspaceEdit,
};

use crate::vault::{Reference, Referenceable, Vault};

pub fn rename(vault: &Vault, params: &RenameParams, path: &Path) -> Option<WorkspaceEdit> {
    let position = params.text_document_position.position;
    let referenceable = vault.select_referenceable_at_position(path, position)?;

    rename_referenceable(vault, &referenceable, params.new_name.clone(), true).map(|ops| {
        WorkspaceEdit {
            document_changes: Some(DocumentChanges::Operations(ops)),
            ..Default::default()
        }
    })
}

pub fn rename_files(vault: &Vault, params: &RenameFilesParams) -> WorkspaceEdit {
    WorkspaceEdit {
        document_changes: Some(DocumentChanges::Operations(
            params
                .files
                .iter()
                .filter_map(|file| {
                    let old_path = Url::from_str(&file.old_uri).ok()?.to_file_path().ok()?;

                    let new_path = Url::from_str(&file.new_uri).ok()?.to_file_path().ok()?;

                    let referenceable = vault.select_referenceable_path(&old_path)?;

                    // should be relative to the current file path
                    let new_name = diff_paths(&new_path, old_path.parent()?)?;
                    let new_name = new_name.to_str()?.to_owned();
                    rename_referenceable(vault, &referenceable, new_name, false)
                })
                .flatten()
                .collect_vec(),
        )),
        ..Default::default()
    }
}

fn rename_referenceable(
    vault: &Vault,
    referenceable: &Referenceable,
    new_name: String,
    move_file: bool,
) -> Option<Vec<DocumentChangeOperation>> {
    let (referenceable_document_change, new_ref_name): (Option<DocumentChangeOperation>, String) =
        match referenceable {
            Referenceable::Heading(path, heading) => {
                let new_text = format!("{} {}", "#".repeat(heading.level.0), new_name); // move this obsidian syntax specific stuff to the vault

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
                    new_name
                );

                (Some(change_op), name.to_string())
            }
            Referenceable::File(path, _file) => {
                let new_path =
                    path_clean::clean(path.with_file_name(&new_name).with_extension("md"));

                // the new name should be the path from the vault root
                let name = diff_paths(&new_path, vault.root_dir()).unwrap();

                let change_op = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                    old_uri: Url::from_file_path(path).ok()?,
                    new_uri: Url::from_file_path(new_path.clone()).ok()?,
                    options: None,
                    annotation_id: None,
                }));

                (
                    move_file.then_some(change_op),
                    name.to_str().unwrap().to_owned(),
                )
            }
            Referenceable::Tag(_path, _tag) => {
                let new_ref_name = new_name.clone();

                let _new_tag = format!("#{}", new_ref_name);

                (None, new_ref_name)
            }
            _ => return None,
        };

    let references = vault.select_references_for_referenceable(referenceable)?;

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

    Some(
        references_changes
            .chain(iter::once(referenceable_document_change).flatten())
            .collect(),
    )
}
