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
    let mut actions = Vec::new();

    let get_checkbox_action = || -> Option<CodeActionOrCommand> {
        let line_chars = vault.select_line(path, params.range.start.line as isize)?;
        let line_str: String = line_chars.into_iter().collect();
        let trimmed = line_str.trim_start();

        let space_idx = trimmed.find(' ')?;
        let (bullet, rest) = trimmed.split_at(space_idx);
        let rest_trimmed = rest.trim_start();

        let is_valid_bullet = bullet == "-"
            || bullet == "*"
            || bullet == "+"
            || (bullet.ends_with('.')
                && bullet.len() > 1
                && bullet[..bullet.len() - 1]
                    .chars()
                    .all(|c| c.is_ascii_digit()));

        if !is_valid_bullet {
            return None;
        }

        // This doesn't check the actual checkbox type, so - [!] will also be toggled
        let has_bracket = rest_trimmed.starts_with('[')
            && rest_trimmed.len() >= 4
            && rest_trimmed.chars().nth(2) == Some(']')
            && rest_trimmed.chars().nth(3) == Some(' ');

        if !has_bracket {
            return None;
        }

        let is_unchecked = rest_trimmed.chars().nth(1) == Some(' ');
        let prefix_bytes = line_str.len() - rest_trimmed.len();
        let char_index = line_str[..prefix_bytes].chars().count();

        let new_char = if is_unchecked { "x" } else { " " };
        let title = if is_unchecked {
            "Toggle checkbox (Check)"
        } else {
            "Toggle checkbox (Uncheck)"
        };

        let uri = Url::from_file_path(path).ok()?;

        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: title.to_string(),
            kind: Some(tower_lsp::lsp_types::CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri,
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit {
                            new_text: new_char.to_string(),
                            range: Range {
                                start: Position {
                                    line: params.range.start.line,
                                    character: (char_index + 1) as u32,
                                },
                                end: Position {
                                    line: params.range.start.line,
                                    character: (char_index + 2) as u32,
                                },
                            },
                        })],
                    }),
                ])),
                ..Default::default()
            }),
            ..Default::default()
        }))
    };

    if settings.checkbox_actions {
        if let Some(action) = get_checkbox_action() {
            actions.push(action);
        }
    }

    // Diagnostics
    // get all links for changed file
    if let Some(unresolved_file_links) = path_unresolved_references(vault, path) {
        let code_action_unresolved = unresolved_file_links.into_iter().filter(|(_, reference)| {
            reference.data().range.start.line <= params.range.start.line
                && reference.data().range.end.line >= params.range.end.line
                && reference.data().range.start.character <= params.range.start.character
                && reference.data().range.end.character >= params.range.end.character
        });

        let mut diagnostic_actions: Vec<CodeActionOrCommand> = code_action_unresolved
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
            .collect();

        actions.append(&mut diagnostic_actions);
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}
