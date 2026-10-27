use std::path::Path;

use pathdiff::diff_paths;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionOrCommand, CodeActionParams, CreateFile, CreateFileOptions,
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, Range, ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

use crate::{
    config::Settings,
    daily::filename_is_formatted,
    diagnostics::path_unresolved_references,
    vault::{Reference, Vault},
};

pub fn code_actions(
    vault: &Vault,
    params: &CodeActionParams,
    path: &Path,
    settings: &Settings,
) -> Option<Vec<CodeActionOrCommand>> {
    // Diagnostics
    // get all links for changed file

    let unresolved = path_unresolved_references(vault, path)?;

    let unresolved_file_links = unresolved;

    let code_action_unresolved = unresolved_file_links.into_iter().filter(|(_, reference)| {
        reference.data().range.start.line <= params.range.start.line
            && reference.data().range.end.line >= params.range.end.line
            && reference.data().range.start.character <= params.range.start.character
            && reference.data().range.end.character >= params.range.end.character
    });

    Some(
        code_action_unresolved
            .flat_map(|(_path, reference)| {
                match reference {
                    Reference::WikiFileLink(_data) => {
                        let filename = &reference.data().reference_text;

                        let mut new_path_buf = vault.root_dir().clone();
                        if filename_is_formatted(settings, filename) {
                            new_path_buf.push(&settings.daily_notes_folder);
                        } else {
                            new_path_buf.push(&settings.new_file_folder_path);
                        }
                        if filename.ends_with(".md") {
                            new_path_buf.push(filename);
                        } else {
                            new_path_buf.push(format!("{filename}.md"));
                        }

                        let new_path = Url::from_file_path(&new_path_buf).ok()?;

                        Some(CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!(
                                "Create File: {:?}",
                                diff_paths(new_path_buf, vault.root_dir())?
                            ),
                            edit: Some(WorkspaceEdit {
                                document_changes: Some(DocumentChanges::Operations(vec![
                                    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                                        uri: new_path,
                                        options: None,
                                        annotation_id: None,
                                    })),
                                ])),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }))
                    }
                    Reference::WikiHeadingLink(_data, link_path, heading) => {

                        let mut new_path_buf = vault.root_dir().clone();
                        if filename_is_formatted(settings, link_path) {
                            new_path_buf.push(&settings.daily_notes_folder);
                        } else {
                            new_path_buf.push(&settings.new_file_folder_path);
                        }
                        if link_path.ends_with(".md") {
                            new_path_buf.push(link_path);
                        } else {
                            new_path_buf.push(format!("{link_path}.md"));
                        }

                        let new_path = Url::from_file_path(&new_path_buf).ok()?;

                        let file = vault.ropes.get(&new_path_buf);

                        let length = match file {
                            Some(file) => file.lines().len(),
                            None => 0
                        };


                        let new_text = match file {
                            Some(..) => format!("\n\n# {}", heading),
                            None => format!("# {}", heading)
                        }; // move this calculation to the vault somehow


                        Some(CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!(
                                "Append Heading \"{}\" to file {}.md, creating it if it doesn't exist",
                                heading,
                                link_path
                            ),
                            edit: Some(WorkspaceEdit{
                                document_changes: Some(DocumentChanges::Operations(vec![
                                    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                                        uri: new_path.clone(),
                                        annotation_id: None,
                                        options: Some(CreateFileOptions {
                                            ignore_if_exists: Some(true),
                                            overwrite: Some(false)
                                        })
                                    })),
                                    DocumentChangeOperation::Edit(TextDocumentEdit{
                                        text_document: OptionalVersionedTextDocumentIdentifier{
                                            uri: new_path,
                                            version: None
                                        },
                                        edits: vec![
                                            OneOf::Left(TextEdit{
                                                new_text,
                                                range: Range {
                                                    start: Position {
                                                        line: (length + 1) as u32,
                                                        character: 0
                                                    },
                                                    end: Position {
                                                        line: length as u32,
                                                        character: 0
                                                    }
                                                }
                                            })
                                        ]
                                    })
                                ])),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }))
                    }
                    _ => None
                }

            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tower_lsp::lsp_types::{
        CodeActionContext, PartialResultParams, TextDocumentIdentifier, WorkDoneProgressParams,
    };

    #[test]
    fn code_action_create_file_preserves_dots_in_note_name() {
        let temp_dir = std::env::temp_dir().join("moxide_test_codeactions_dots");
        let _ = std::fs::create_dir_all(&temp_dir);
        let settings = Settings::new(&temp_dir, &Default::default()).unwrap();
        let mut vault = Vault::construct_vault(&settings, &temp_dir).unwrap();

        let source_path = temp_dir.join("index.md");
        Vault::update_vault(
            &settings,
            &mut vault,
            (&source_path, "[[TECH.51.04-Proxmox backups]]"),
        );

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(&source_path).unwrap(),
            },
            range: Range {
                start: Position {
                    line: 0,
                    character: 2,
                },
                end: Position {
                    line: 0,
                    character: 2,
                },
            },
            context: CodeActionContext::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let actions = code_actions(&vault, &params, &source_path, &settings).unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            CodeActionOrCommand::CodeAction(action) => {
                assert_eq!(
                    action.title,
                    format!(
                        "Create File: {:?}",
                        PathBuf::from("TECH.51.04-Proxmox backups.md")
                    )
                );
                let Some(DocumentChanges::Operations(ops)) = action
                    .edit
                    .as_ref()
                    .and_then(|e| e.document_changes.as_ref())
                else {
                    panic!("expected DocumentChanges::Operations");
                };
                let DocumentChangeOperation::Op(ResourceOp::Create(create_file)) = &ops[0] else {
                    panic!("expected CreateFile operation");
                };
                assert_eq!(
                    create_file.uri.to_file_path().unwrap(),
                    temp_dir.join("TECH.51.04-Proxmox backups.md")
                );
            }
            _ => panic!("expected CodeAction"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn code_action_append_heading_preserves_dots_in_note_name() {
        let temp_dir = std::env::temp_dir().join("moxide_test_codeactions_heading_dots");
        let _ = std::fs::create_dir_all(&temp_dir);
        let settings = Settings::new(&temp_dir, &Default::default()).unwrap();
        let mut vault = Vault::construct_vault(&settings, &temp_dir).unwrap();

        let source_path = temp_dir.join("index.md");
        Vault::update_vault(
            &settings,
            &mut vault,
            (&source_path, "[[TECH.51.04-Proxmox backups#Setup]]"),
        );

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(&source_path).unwrap(),
            },
            range: Range {
                start: Position {
                    line: 0,
                    character: 2,
                },
                end: Position {
                    line: 0,
                    character: 2,
                },
            },
            context: CodeActionContext::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let actions = code_actions(&vault, &params, &source_path, &settings).unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            CodeActionOrCommand::CodeAction(action) => {
                let Some(DocumentChanges::Operations(ops)) = action
                    .edit
                    .as_ref()
                    .and_then(|e| e.document_changes.as_ref())
                else {
                    panic!("expected DocumentChanges::Operations");
                };
                let DocumentChangeOperation::Op(ResourceOp::Create(create_file)) = &ops[0] else {
                    panic!("expected CreateFile operation");
                };
                assert_eq!(
                    create_file.uri.to_file_path().unwrap(),
                    temp_dir.join("TECH.51.04-Proxmox backups.md")
                );
            }
            _ => panic!("expected CodeAction"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
