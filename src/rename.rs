use std::iter;
use std::path::Path;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, RenameFile, RenameFileOptions, RenameParams,
    ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

use crate::vault::{MDHeading, Reference, ReferenceData, Referenceable, Vault};

pub fn rename(vault: &Vault, params: &RenameParams, path: &Path) -> Option<WorkspaceEdit> {
    let position = params.text_document_position.position;
    let referenceable = vault.select_referenceable_at_position(path, position)?;

    let (referenceable_document_change, new_ref_name): (DocumentChangeOperation, String) =
        match referenceable {
            Referenceable::Heading(path, heading) => {
                let new_text = format!("{} {}", "#".repeat(heading.level.0), params.new_name); // move this obsidian syntax specific stuff to the vault

                let change_op = DocumentChangeOperation::Edit(TextDocumentEdit {
                    text_document: tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                        uri: Url::from_file_path(path).ok()?,
                        version: None,
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        range: heading.range,
                        new_text,
                    })],
                });

                let name = Referenceable::Heading(
                    path,
                    &MDHeading {
                        heading_text: params.new_name.clone(),
                        ..heading.clone()
                    },
                )
                .get_refname(&vault.root_dir())?;

                (change_op, name)
            }
            Referenceable::File(path, file) => {
                let new_path = path.with_file_name(&params.new_name).with_extension("md");

                let change_op = DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                    old_uri: Url::from_file_path(path).ok()?,
                    new_uri: Url::from_file_path(new_path.clone()).ok()?,
                    options: None,
                    annotation_id: None,
                }));

                let name = Referenceable::File(&new_path, file).get_refname(&vault.root_dir())?;

                (change_op, name)
            }
            _ => return None,
        };

    let references = vault.select_references_for_referenceable(&referenceable)?;

    let references_changes = references
        .into_iter()
        .filter_map(|(path, reference)| {
            // update references

            match reference {
                Reference::Link(data) => {
                    let new_ref_name = match data.reference_text.split_once("#") {
                        Some((file, rest))
                            if matches!(referenceable, Referenceable::File(_, _)) =>
                        {
                            format!("{}#{}", new_ref_name, rest)
                        }
                        _ => new_ref_name.clone(),
                    };

                    let new_text = format!(
                        "[[{}{}]]",
                        new_ref_name,
                        data.display_text
                            .as_ref()
                            .map(|text| format!("|{text}"))
                            .unwrap_or_else(|| String::from(""))
                    );

                    return Some(TextDocumentEdit {
                        text_document:
                            tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier {
                                uri: Url::from_file_path(path).ok()?,
                                version: None,
                            },
                        edits: vec![OneOf::Left(TextEdit {
                            range: data.range,
                            new_text: new_text,
                        })],
                    });
                }
                _ => None,
            }
        })
        .map(|edit| DocumentChangeOperation::Edit(edit));

    return Some(WorkspaceEdit {
        document_changes: Some(DocumentChanges::Operations(
            references_changes
                .chain(iter::once(referenceable_document_change))
                .collect(), // order matters here
        )),
        ..Default::default()
    });
}
