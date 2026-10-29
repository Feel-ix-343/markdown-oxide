use std::collections::HashMap;
use std::path::Path;

use pathdiff::diff_paths;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CreateFile, CreateFileOptions,
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, Range, ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};

use crate::{
    config::Settings,
    daily::filename_is_formatted,
    diagnostics::path_unresolved_references,
    vault::{heading_to_slug, MDHeading, Reference, Vault},
};

pub fn generate_toc_markdown(headings: &[MDHeading], settings: &Settings) -> String {
    let min_level = headings.iter().map(|h| h.level.0).min().unwrap_or(1);
    let mut lines = Vec::new();
    for heading in headings {
        let depth = heading.level.0.saturating_sub(min_level);
        let indent = "  ".repeat(depth);
        let target = if settings.heading_slug {
            format!("#{}", heading_to_slug(&heading.heading_text))
        } else if heading.heading_text.contains(' ') {
            format!("<#{}>", heading.heading_text)
        } else {
            format!("#{}", heading.heading_text)
        };
        lines.push(format!("{}- [{}]({})", indent, heading.heading_text, target));
    }
    lines.join("\n") + "\n"
}

pub fn generate_toc_wikilink(headings: &[MDHeading]) -> String {
    let min_level = headings.iter().map(|h| h.level.0).min().unwrap_or(1);
    let mut lines = Vec::new();
    for heading in headings {
        let depth = heading.level.0.saturating_sub(min_level);
        let indent = "  ".repeat(depth);
        lines.push(format!("{}- [[#{}]]", indent, heading.heading_text));
    }
    lines.join("\n") + "\n"
}

pub fn code_actions(
    vault: &Vault,
    params: &CodeActionParams,
    path: &Path,
    settings: &Settings,
) -> Option<Vec<CodeActionOrCommand>> {
    let mut actions: Vec<CodeActionOrCommand> = Vec::new();

    // 1. Diagnostics / Unresolved References Code Actions
    if let Some(unresolved) = path_unresolved_references(vault, path) {
        let code_action_unresolved = unresolved.into_iter().filter(|(_, reference)| {
            reference.data().range.start.line <= params.range.start.line
                && reference.data().range.end.line >= params.range.end.line
                && reference.data().range.start.character <= params.range.start.character
                && reference.data().range.end.character >= params.range.end.character
        });

        for (_path, reference) in code_action_unresolved {
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

                    if let Ok(new_path) = Url::from_file_path(&new_path_buf) {
                        if let Some(diff) = diff_paths(&new_path_buf, vault.root_dir()) {
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                                title: format!("Create File: {:?}", diff),
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
                            }));
                        }
                    }
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

                    if let Ok(new_path) = Url::from_file_path(&new_path_buf) {
                        let file = vault.ropes.get(&new_path_buf);
                        let length = match file {
                            Some(file) => file.lines().len(),
                            None => 0,
                        };

                        let new_text = match file {
                            Some(..) => format!("\n\n# {}", heading),
                            None => format!("# {}", heading),
                        };

                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!(
                                "Append Heading \"{}\" to file {}.md, creating it if it doesn't exist",
                                heading, link_path
                            ),
                            edit: Some(WorkspaceEdit {
                                document_changes: Some(DocumentChanges::Operations(vec![
                                    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                                        uri: new_path.clone(),
                                        annotation_id: None,
                                        options: Some(CreateFileOptions {
                                            ignore_if_exists: Some(true),
                                            overwrite: Some(false),
                                        }),
                                    })),
                                    DocumentChangeOperation::Edit(TextDocumentEdit {
                                        text_document: OptionalVersionedTextDocumentIdentifier {
                                            uri: new_path,
                                            version: None,
                                        },
                                        edits: vec![OneOf::Left(TextEdit {
                                            new_text,
                                            range: Range {
                                                start: Position {
                                                    line: (length + 1) as u32,
                                                    character: 0,
                                                },
                                                end: Position {
                                                    line: length as u32,
                                                    character: 0,
                                                },
                                            },
                                        })],
                                    }),
                                ])),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }));
                    }
                }
                _ => {}
            }
        }
    }

    // 2. Table of Contents CodeActions
    if let Some(headings) = vault.select_headings(path) {
        if !headings.is_empty() {
            if let Ok(url) = Url::from_file_path(path) {
                let insert_pos = Position {
                    line: params.range.start.line,
                    character: 0,
                };
                let insert_range = Range {
                    start: insert_pos,
                    end: insert_pos,
                };

                let mut changes_md = HashMap::new();
                changes_md.insert(
                    url.clone(),
                    vec![TextEdit {
                        range: insert_range,
                        new_text: generate_toc_markdown(headings, settings),
                    }],
                );

                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Generate Table of Contents (Markdown Links)".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes_md),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));

                let mut changes_wiki = HashMap::new();
                changes_wiki.insert(
                    url,
                    vec![TextEdit {
                        range: insert_range,
                        new_text: generate_toc_wikilink(headings),
                    }],
                );

                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Generate Table of Contents (WikiLinks)".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes_wiki),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }
    }

    Some(actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{HeadingLevel, MyRange};
    use std::fs;
    use tower_lsp::lsp_types::{ClientCapabilities, CodeActionContext, CodeActionParams, Position, Range};

    #[test]
    fn test_generate_toc_markdown_formatting() {
        let settings_default = Settings::new(&std::env::temp_dir(), &ClientCapabilities::default()).unwrap();
        let headings = vec![
            MDHeading {
                heading_text: "Introduction".to_string(),
                range: MyRange(Range::default()),
                level: HeadingLevel(1),
            },
            MDHeading {
                heading_text: "Nested Topic".to_string(),
                range: MyRange(Range::default()),
                level: HeadingLevel(2),
            },
            MDHeading {
                heading_text: "Deep Topic".to_string(),
                range: MyRange(Range::default()),
                level: HeadingLevel(3),
            },
            MDHeading {
                heading_text: "Conclusion".to_string(),
                range: MyRange(Range::default()),
                level: HeadingLevel(1),
            },
        ];

        let toc_md = generate_toc_markdown(&headings, &settings_default);
        assert_eq!(
            toc_md,
            "- [Introduction](#Introduction)\n  - [Nested Topic](<#Nested Topic>)\n    - [Deep Topic](<#Deep Topic>)\n- [Conclusion](#Conclusion)\n"
        );

        let mut settings_slug = settings_default.clone();
        settings_slug.heading_slug = true;
        let toc_slug = generate_toc_markdown(&headings, &settings_slug);
        assert_eq!(
            toc_slug,
            "- [Introduction](#Introduction)\n  - [Nested Topic](#Nested-Topic)\n    - [Deep Topic](#Deep-Topic)\n- [Conclusion](#Conclusion)\n"
        );
    }

    #[test]
    fn test_generate_toc_wikilink_formatting() {
        let headings = vec![
            MDHeading {
                heading_text: "Introduction".to_string(),
                range: MyRange(Range::default()),
                level: HeadingLevel(1),
            },
            MDHeading {
                heading_text: "Nested Topic".to_string(),
                range: MyRange(Range::default()),
                level: HeadingLevel(2),
            },
        ];

        let toc_wiki = generate_toc_wikilink(&headings);
        assert_eq!(
            toc_wiki,
            "- [[#Introduction]]\n  - [[#Nested Topic]]\n"
        );
    }

    #[test]
    fn test_code_action_table_of_contents_available_when_headings_exist() {
        let unique_dir = format!("test_moxide_toc_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let root = std::env::temp_dir().join(unique_dir);
        fs::create_dir_all(&root).unwrap();
        let file_path = root.join("test.md");
        fs::write(&file_path, "# Main Title\n\nSome text.\n\n## Sub Topic\n\nMore text.\n").unwrap();

        let settings = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        let vault = Vault::construct_vault(&settings, &root).unwrap();

        let params = CodeActionParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier {
                uri: Url::from_file_path(&file_path).unwrap(),
            },
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            context: CodeActionContext::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let actions = code_actions(&vault, &params, &file_path, &settings).unwrap();
        let titles: Vec<String> = actions.into_iter().filter_map(|a| match a {
            CodeActionOrCommand::CodeAction(ca) => Some(ca.title),
            _ => None,
        }).collect();

        assert!(titles.contains(&"Generate Table of Contents (Markdown Links)".to_string()));
        assert!(titles.contains(&"Generate Table of Contents (WikiLinks)".to_string()));

        let _ = fs::remove_dir_all(&root);
    }
}
