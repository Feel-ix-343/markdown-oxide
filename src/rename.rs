use std::iter;
use std::path::Path;

use tower_lsp::lsp_types::{RenameParams, WorkspaceEdit, DocumentChanges, DocumentChangeOperation, TextDocumentEdit, OneOf, TextEdit, Url};
use tower_lsp::jsonrpc::Result;

use crate::vault::{Vault, Reference, ReferenceData, Referenceable, MDHeading};


pub fn rename(vault: &Vault, params: &RenameParams, path: &Path) -> Option<WorkspaceEdit> {
    let position = params.text_document_position.position;
    let referenceable = vault.select_referenceable_at_position(path, position)?;

    let referenceable_document_change: DocumentChangeOperation = match referenceable {
        Referenceable::Heading(_, heading) => {
            let new_text = format!("{} {}", "#".repeat(heading.level.0), params.new_name); // move this obsidian syntax specific stuff to the vault

            DocumentChangeOperation::Edit(TextDocumentEdit { 
                text_document: tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier { uri: Url::from_file_path(path).ok()?, version: None},
                edits: vec![
                    OneOf::Left(
                    TextEdit {
                        range: heading.range,
                        new_text 
                    }
                    )
                ]
            })
        },
        _ => return None
    };


    let new_ref_name = match referenceable {
        Referenceable::Heading(path, heading) => {
            Referenceable::Heading(path, &MDHeading {heading_text: params.new_name.clone(), ..heading.clone()}).get_refname(&vault.root_dir())?
        },
        _ => return None
    };

    let references = vault.select_references_for_referenceable(&referenceable)?;

    let references_changes = references.into_iter()
        .filter_map(|(path, reference)|  {
            // update references

            match reference {
                Reference::Link(data) => {
                    let new_text = format!("[[{}{}]]", new_ref_name, data.display_text.as_ref().map(|text| format!("|{text}")).unwrap_or_else(|| String::from("")));

                    return Some(TextDocumentEdit {
                        text_document: tower_lsp::lsp_types::OptionalVersionedTextDocumentIdentifier { uri: Url::from_file_path(path).ok()?, version: None},
                        edits: vec![
                            OneOf::Left(

                            TextEdit {
                            range: data.range,
                            new_text: new_text
                            }
                            )
                        ]
                    })
                },
                    _ => None
            }
        })
        .map(|edit| DocumentChangeOperation::Edit(edit));


    return Some(WorkspaceEdit {
        document_changes: Some(DocumentChanges::Operations(
            iter::once(referenceable_document_change).chain(references_changes).collect()
        )),
        ..Default::default()
    })

}
