use std::path::Path;

use tower_lsp::lsp_types::{
    Command as LspCommand, CompletionItem, CompletionItemKind, CompletionItemLabelDetails,
    CompletionList, CompletionResponse, CompletionTextEdit, Documentation, MarkupContent, TextEdit,
    Url,
};

use nanoid::nanoid;
use vault::Vault;

use crate::{
    command::{
        actions::{Actions, Edit},
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

            let binding = actions.actions().first().unwrap().edits();
            let Edit {
                new_text,
                start,
                end,
            } = binding.iter().next().unwrap().1.first().unwrap();
            let text_edit = TextEdit {
                new_text: new_text.to_string(),
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
            };

            let filter_text = {
                let text_to_cursor = cmd_displayer.range_to_cursor(
                    info.path,
                    info.line,
                    text_edit.range.start.character,
                    info.cursor,
                );
                let label = label.to_string();
                format!("{text_to_cursor}{label}")
            };
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
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                filter_text: Some(filter_text),
                sort_text: Some(i.to_string()),
                label_details: try {
                    let label_detail = label_detail?;
                    CompletionItemLabelDetails {
                        detail: Some(label_detail),
                        description: None,
                    }
                },
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
}

impl<'a> CmdDisplayer<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}
