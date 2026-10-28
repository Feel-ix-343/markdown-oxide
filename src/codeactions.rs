use std::path::Path;

use pathdiff::diff_paths;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionOrCommand, CodeActionParams, Command, CreateFile, CreateFileOptions,
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
                        new_path_buf.push(filename);
                        new_path_buf.set_extension("md");

                        let new_path = Url::from_file_path(&new_path_buf).ok()?;

                        let command = if settings.open_created_file {
                            Some(Command {
                                title: "Open created note".to_string(),
                                command: "moxide.open_file".to_string(),
                                arguments: Some(vec![serde_json::to_value(&new_path).ok()?]),
                            })
                        } else {
                            None
                        };

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
                            command,
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
                        new_path_buf.push(link_path);
                        new_path_buf.set_extension("md");

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


                        let command = if settings.open_created_file {
                            Some(Command {
                                title: "Open created note".to_string(),
                                command: "moxide.open_file".to_string(),
                                arguments: Some(vec![serde_json::to_value(&new_path).ok()?]),
                            })
                        } else {
                            None
                        };

                        Some(CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!(
                                "Append Heading \"{}\" to file {}.md, creating it if it doesn't exist",
                                heading,
                                link_path
                            ),
                            command,
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
    use std::fs;
    use tower_lsp::lsp_types::{ClientCapabilities, CodeActionContext, CodeActionParams, Position, Range};

    #[test]
    fn test_code_action_open_created_file_flag() {
        let unique_dir = format!("test_moxide_ca_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let root = std::env::temp_dir().join(unique_dir);
        fs::create_dir_all(&root).unwrap();
        let file_path = root.join("test.md");
        fs::write(&file_path, "See [[new_note]]").unwrap();

        let mut settings = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        let vault = Vault::construct_vault(&settings, &root).unwrap();

        let params = CodeActionParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier {
                uri: Url::from_file_path(&file_path).unwrap(),
            },
            range: Range {
                start: Position { line: 0, character: 5 },
                end: Position { line: 0, character: 15 },
            },
            context: CodeActionContext::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        // When open_created_file is true (default)
        settings.open_created_file = true;
        let actions = code_actions(&vault, &params, &file_path, &settings).unwrap();
        assert_eq!(actions.len(), 1);
        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert!(action.command.is_some());
            let cmd = action.command.as_ref().unwrap();
            assert_eq!(cmd.command, "moxide.open_file");
        } else {
            panic!("Expected CodeAction");
        }

        // When open_created_file is false
        settings.open_created_file = false;
        let actions_disabled = code_actions(&vault, &params, &file_path, &settings).unwrap();
        assert_eq!(actions_disabled.len(), 1);
        if let CodeActionOrCommand::CodeAction(action) = &actions_disabled[0] {
            assert!(action.command.is_none());
        } else {
            panic!("Expected CodeAction");
        }

        let _ = fs::remove_dir_all(&root);
    }
}
