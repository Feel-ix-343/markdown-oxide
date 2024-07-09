#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(try_blocks)]
#![allow(clippy::map_flatten)]

mod cmd_displayer;
mod command;
mod context;
use anyhow::anyhow;
pub use cache::QueryCache;
pub use context::QueryContext;
use tower_lsp::lsp_types::DidChangeTextDocumentParams;

mod cache;
mod entity;
mod entity_viewer;
mod parser;
mod querier;
mod settings;

use std::path::Path;
use std::sync::Arc;

use cmd_displayer::cmds_lsp_comp_resp;
use cmd_displayer::Input;
use parser::BlockLinkCmdQuery;
use parser::NamedRefCmdQuery;
use querier::query_block_link_cmds;
use querier::query_named_ref_cmds;
use tower_lsp::lsp_types::{CompletionParams, CompletionResponse};

pub fn get_completions(
    params: &CompletionParams,
    path: &Path,
    cx: QueryContext,
) -> Option<CompletionResponse> {
    let location = Location {
        path,
        line: params.text_document_position.position.line,
        character: params.text_document_position.position.character,
    };

    completions(cx, location)
}

fn completions<'fs: 'cache, 'cache>(
    mut cx: QueryContext<'fs, 'cache>,
    location: Location,
) -> Option<CompletionResponse> {
    if let Some((block_link_cmd_query, meta)) = cx.parser().parse_block_query(location)
    // This should give parser a mutable reference to its cache data with a lifetime of cache, while also giving parser
    // access to vault with a lifetime of file system.
    {
        cx.cache().previous_metadata = Some(meta.clone());
        let vault_blocks = cx.cache().querier_cache.blocks.clone();
        let vault_blocks = match vault_blocks {
            None => Arc::new(cx.querier().get_blocks(&meta)),
            Some(named) => named,
        };
        let unnamed_entities =
            query_block_link_cmds(&cx, &meta, &block_link_cmd_query, &vault_blocks);
        let r = Some(cmds_lsp_comp_resp(
            &cx,
            &meta,
            unnamed_entities,
            &block_link_cmd_query,
        ));

        cx.cache().querier_cache.blocks = Some(vault_blocks);

        r
    } else if let Some((ref_cmds_query, meta)) = cx.parser().parse_entity_query(location) {
        cx.cache().previous_metadata = Some(meta.clone());
        let named_sections = cx.cache().querier_cache.named_sections.clone();
        let named_sections = match named_sections {
            None => Arc::new(cx.querier().get_named_sections()),
            Some(named) => named,
        };
        let named_entities = query_named_ref_cmds(&cx, &meta, &ref_cmds_query, &named_sections);
        let result = Some(cmds_lsp_comp_resp(
            &cx,
            &meta,
            named_entities,
            &ref_cmds_query,
        ));

        cx.cache().querier_cache.named_sections = Some(named_sections);

        result
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

impl Input for BlockLinkCmdQuery {
    fn grep_filter(&self) -> std::string::String {
        self.grep_string().to_string()
    }
}

impl Input for NamedRefCmdQuery<'_> {
    fn grep_filter(&self) -> std::string::String {
        self.grep_string()
    }
}

pub fn lsp_sync(
    mut cx: QueryContext<'_, '_>,
    lsp_did_change: DidChangeTextDocumentParams,
) -> anyhow::Result<CacheResult> {
    // clear cache if change happens outside of previous query
    let Some(ref meta) = cx.cache().previous_metadata else {
        return Ok(CacheResult::Unchanged);
    };

    let path = lsp_did_change
        .text_document
        .uri
        .to_file_path()
        .or(Err(anyhow!("Failed to unwrap did_change_path")))?;

    if lsp_did_change.content_changes.iter().any(|change| {
        change.range.is_none()
            || change.range.is_some_and(|range| {
                range.start.line != meta.line
                    || range.end.line != meta.line
                    || (range.start.line == meta.line
                        && range.start.character < meta.char_range.start as u32)
                    || (range.end.line == meta.line
                        && range.end.character > meta.char_range.end as u32)
            })
    }) || path != meta.path
    {
        cx.cache().clear();
        Ok(CacheResult::Cleared)
    } else {
        Ok(CacheResult::Unchanged)
    }
}

#[derive(Debug)]
pub enum CacheResult {
    Cleared,
    Unchanged,
    IsNone,
}
