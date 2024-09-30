use std::{path::Path, sync::Arc};

use anyhow::{anyhow, Context};
use md_parser::{DocBlock, Document, ListBlock};
use serde::{Deserialize, Serialize};

use crate::mem_fs;

pub type Line = usize;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct File {}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Heading {
    pub title: String,
    pub range: std::ops::Range<Line>,
    /// Full range of the section that heading belongs to
    pub full_range: std::ops::Range<Line>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Block {
    pub range: std::ops::Range<Line>,
    /// Range of sub-blocks, if any
    pub context_range: Option<ContextRange>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ContextRange {
    pub parent: Option<std::ops::Range<Line>>,
    pub children: Option<std::ops::Range<Line>>,
}

pub fn parse_file(
    relative_path: &str,
    fs_file: &mem_fs::FsFile,
) -> anyhow::Result<(File, Vec<Heading>, Vec<Block>)> {
    let file = File {};

    let document = md_parser::Document::new(&fs_file.text())
        .context(format!("Parsing document {relative_path:?}"))?;

    let headings = document
        .sections()
        .filter_map(|section| {
            section.heading.as_ref().map(|heading| Heading {
                title: heading.text.to_string(),
                range: heading.range.start_point.row..heading.range.end_point.row,
                full_range: section.range.start_point.row..section.range.end_point.row,
            })
        })
        .collect::<Vec<_>>();

    fn recurse_list_block<'a>(
        block: &'a ListBlock,
        parent: Option<&ListBlock>,
    ) -> Box<dyn Iterator<Item = Block> + 'a> {
        let parent_range = parent
            .map(|block| block.content.range.start_point.row..block.content.range.end_point.row);
        let children_range = block.children.as_ref().and_then(|children| {
            let first = children.first()?;
            let last = children.last()?;
            Some(first.range.start_point.row..last.range.end_point.row)
        });

        let md_block = Block {
            range: block.range.start_point.row..block.range.end_point.row,
            context_range: Some(ContextRange {
                parent: parent_range,
                children: children_range,
            }),
        };

        let children_blocks = block
            .children
            .iter()
            .flatten()
            .map(|child| recurse_list_block(child, Some(block)))
            .flatten();

        Box::new(std::iter::once(md_block).chain(children_blocks))
    }

    let blocks = document
        .sections()
        .map(|section| section.top_level_blocks())
        .flatten()
        .map(|block| match block {
            DocBlock::ListBlock(block) => recurse_list_block(block, None),
            DocBlock::ParagraphBlock(block) => Box::new(std::iter::once(Block {
                context_range: None,
                range: block.content.range.start_point.row..block.content.range.end_point.row,
            })),
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok((file, headings, blocks))
}
