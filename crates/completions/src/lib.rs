#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(try_blocks)]
#![allow(clippy::map_flatten)]

mod cmd_displayer;
mod command;
mod context;
mod entity;
mod entity_viewer;
mod parser;
mod querier;
mod settings;

use std::path::{Path, PathBuf};

use cmd_displayer::{cmds_lsp_comp_resp, CmdDisplayer};
use context::Context;
use entity_viewer::EntityViewer;
use moxide_config::Settings;
use parser::Parser;
use querier::{query_block_link_cmds, query_named_ref_cmds, Querier};
use settings::SettingsAdapter;
use tower_lsp::lsp_types::{CompletionParams, CompletionResponse};
use vault::Vault;

pub fn get_completions(
    vault: &Vault,
    _files: &[PathBuf],
    params: &CompletionParams,
    path: &Path,
    settings: &Settings,
) -> Option<CompletionResponse> {
    let cx = Context::new(
        Parser::new(vault),
        Querier::new(vault),
        SettingsAdapter::new(settings, vault),
        EntityViewer::new(vault),
        CmdDisplayer::new(vault),
    );

    let location = Location {
        path,
        line: params.text_document_position.position.line,
        character: params.text_document_position.position.character,
    };

    completions(&cx, location)
}

fn completions(cx: &Context, location: Location) -> Option<CompletionResponse> {
    if let Some((block_link_cmd_query, query_syntax_info)) = cx.parser().parse_block_query(location)
    {
        let unnamed_entities = query_block_link_cmds(cx, &query_syntax_info, &block_link_cmd_query);
        Some(cmds_lsp_comp_resp(cx, &query_syntax_info, unnamed_entities))
    } else if let Some((ref_cmds_query, query_syntax_info)) =
        cx.parser().parse_entity_query(location)
    {
        let named_entities = query_named_ref_cmds(cx, &query_syntax_info, &ref_cmds_query);
        Some(cmds_lsp_comp_resp(cx, &query_syntax_info, named_entities))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
struct Location<'fs> {
    path: &'fs Path,
    line: u32,
    character: u32,
}
