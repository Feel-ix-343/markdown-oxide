use std::{collections::HashMap, iter, path::Path};

use tower_lsp::lsp_types::{
    Command as LspCommand, CompletionItem, CompletionItemKind, CompletionItemLabelDetails,
    CompletionList, CompletionResponse, CompletionTextEdit, Documentation, MarkupContent, TextEdit,
    Url, WorkspaceEdit,
};

use itertools::Itertools;
use nanoid::nanoid;
use vault::Vault;

use crate::{
    command::{
        actions::{Actions, Edit, EditArea},
        Command,
    },
    entity::{Block, Entity, EntityInfo, NamedEntityTypeInfo},
    entity_viewer::EntityViewer,
    parser::{QueryMetadata, QuerySyntaxInfo, QuerySyntaxTypeInfo},
    settings::SettingsAdapter,
    Context,
};

use rayon::prelude::*;

pub fn cmds_lsp_comp_resp<A: Actions>(
    cx: &Context,
    info: &QueryMetadata,
    cmds: impl IndexedParallelIterator<Item = Command<A>>,
) -> CompletionResponse {
    let cmd_displayer = cx.cmd_displayer();
    let items: Vec<CompletionItem> = cmds
        .enumerate()
        .map(|(i, cmd)| {
            let Command {
                label,
                kind,
                cmd_ui_info,
                actions,
                label_detail,
            } = cmd;

            let (text_edits_iter, workspace_edit_hmap_iter) = actions
                .actions()
                .into_iter()
                .map(|action| action.edits())
                .flatten()
                .fold(
                    (
                        Box::new(iter::empty()) as Box<dyn Iterator<Item = TextEdit>>,
                        Box::new(iter::empty()) as Box<dyn Iterator<Item = (Url, Vec<TextEdit>)>>,
                    ),
                    |(text_edit_acc, workspace_edit_acc), (path, edits)| {
                        let text_edits = edits
                            .iter()
                            .map(|edit| match edit {
                                Edit {
                                    insert_text,
                                    to_area: EditArea::Range { start, end },
                                } => TextEdit {
                                    new_text: insert_text.to_string(),
                                    range: tower_lsp::lsp_types::Range {
                                        start: tower_lsp::lsp_types::Position {
                                            line: start.line,
                                            character: start.character,
                                        },
                                        end: tower_lsp::lsp_types::Position {
                                            line: end.line,
                                            character: end.character,
                                        },
                                    },
                                },
                                Edit {
                                    insert_text,
                                    to_area: EditArea::EndOfLine(line),
                                } => {
                                    let last_character =
                                        cx.cmd_displayer().get_chars_in_line(path, *line) - 1;
                                    TextEdit {
                                        new_text: insert_text.to_string(),
                                        range: tower_lsp::lsp_types::Range {
                                            start: tower_lsp::lsp_types::Position {
                                                line: *line,
                                                character: last_character,
                                            },
                                            end: tower_lsp::lsp_types::Position {
                                                line: *line,
                                                character: last_character,
                                            },
                                        },
                                    }
                                }
                            })
                            .collect();

                        if path == info.path {
                            let text_edit_acc = text_edit_acc.chain(text_edits);
                            (Box::new(text_edit_acc), workspace_edit_acc)
                        } else {
                            let workspace_edit_acc = workspace_edit_acc.chain(iter::once((
                                Url::from_file_path(path).expect("Path should convert"),
                                text_edits,
                            )));
                            (text_edit_acc, Box::new(workspace_edit_acc))
                        }
                    },
                );
            let changes: HashMap<_, _> = workspace_edit_hmap_iter.collect();
            let workspace_edit: Option<WorkspaceEdit> = if !changes.is_empty() {
                Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                })
            } else {
                None
            };
            // take the text edit surrounding the cursor
            let (text_edit, addtl_edits): (Option<TextEdit>, Box<dyn Iterator<Item = TextEdit>>) =
                text_edits_iter.fold((None, Box::new(iter::empty())), |(te, adlte), edit| {
                    if edit.range.start.line <= info.line
                        && edit.range.end.line >= info.line
                        && edit.range.start.character <= info.cursor
                        && edit.range.end.character >= info.cursor
                    {
                        (Some(edit), adlte)
                    } else {
                        (te, Box::new(adlte.chain(iter::once(edit))))
                    }
                });
            let addtl_edits = addtl_edits.collect::<Vec<_>>();

            let filter_text: Option<String> = text_edit.as_ref().map(|it| {
                let text_to_cursor = cmd_displayer.range_to_cursor(
                    info.path,
                    info.line,
                    it.range.start.character,
                    info.cursor,
                );
                let label = label.to_string();
                format!("{text_to_cursor}{label}")
            });
            CompletionItem {
                label,
                kind: Some(kind),
                documentation: try {
                    let value = cmd_ui_info?;
                    Documentation::MarkupContent(MarkupContent {
                        kind: tower_lsp::lsp_types::MarkupKind::Markdown,
                        value,
                    })
                },
                text_edit: text_edit.map(CompletionTextEdit::Edit),
                additional_text_edits: if addtl_edits.is_empty() {
                    None
                } else {
                    Some(addtl_edits)
                },
                filter_text,
                sort_text: Some(i.to_string()),
                label_details: try {
                    let label_detail = label_detail?;
                    CompletionItemLabelDetails {
                        detail: Some(label_detail),
                        description: None,
                    }
                },
                command: workspace_edit.map(|it| LspCommand {
                    command: "apply_edits".to_string(),
                    arguments: Some(vec![serde_json::to_value(it).unwrap()]),
                    title: "Edit file".to_string(),
                }),
                ..Default::default()
            }
        })
        .collect();

    CompletionResponse::List(CompletionList {
        items,
        is_incomplete: true,
    })
}

pub struct CmdDisplayer<'a> {
    vault: &'a Vault,
}

impl<'a> CmdDisplayer<'a> {
    fn range_to_cursor(
        &self,
        file: &Path,
        line: u32,
        text_edit_range_start: u32,
        cursor: u32,
    ) -> String {
        self.vault.select_line(file, line as isize).unwrap()
            [text_edit_range_start as usize..cursor as usize]
            .iter()
            .collect()
    }

    fn get_chars_in_line(&self, path: &Path, line: u32) -> u32 {
        self.vault.select_line(path, line as isize).unwrap().len() as u32
    }
}

impl<'a> CmdDisplayer<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}
