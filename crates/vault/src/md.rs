use std::{path::Path, sync::Arc};

use md_parser::{DocBlock, Document, ListBlock};
use serde::{Deserialize, Serialize};

use crate::mem_fs;

pub type Line = usize;

#[derive(Serialize, Deserialize, Debug)]
pub struct File {
    pub content_range: std::ops::Range<Line>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Heading {
    pub title: String,
    pub range: std::ops::Range<Line>,
    /// Full range of the section that heading belongs to
    pub full_range: std::ops::Range<Line>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    pub range: std::ops::Range<Line>,
    /// Range of sub-blocks, if any
    pub context_range: Option<ContextRange>,
}
#[derive(Serialize, Deserialize, Debug)]
struct ContextRange {
    pub parent: Option<std::ops::Range<Line>>,
    pub children: Option<std::ops::Range<Line>>,
}

pub struct ParsedFile(pub Arc<File>, pub Vec<Arc<Heading>>, pub Vec<Arc<Block>>);

impl ParsedFile {
    pub fn construct(path: &Path, rope: &ropey::Rope, document: &Document) -> anyhow::Result<Self> {
        let file = Arc::new(File {
            content_range: 0..rope.len_lines(),
        });

        let headings = document
            .sections()
            .filter_map(|section| {
                section.heading.as_ref().map(|heading| {
                    Arc::new(Heading {
                        title: heading.text.to_string(),
                        range: heading.range.start_point.row..heading.range.end_point.row,
                        full_range: section.range.start_point.row..section.range.end_point.row,
                    })
                })
            })
            .collect();

        fn recurse_list_block<'a>(
            block: &'a ListBlock,
            parent: Option<&ListBlock>,
        ) -> Box<dyn Iterator<Item = Arc<Block>> + 'a> {
            let parent_range = parent.map(|block| {
                block.content.range.start_point.row..block.content.range.end_point.row
            });
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

            Box::new(std::iter::once(Arc::new(md_block)).chain(children_blocks))
        }

        let blocks = document
            .sections()
            .map(|section| section.top_level_blocks())
            .flatten()
            .map(|block| match block {
                DocBlock::ListBlock(block) => recurse_list_block(block, None),
                DocBlock::ParagraphBlock(block) => Box::new(std::iter::once(Arc::new(Block {
                    context_range: None,
                    range: block.content.range.start_point.row..block.content.range.end_point.row,
                }))),
            })
            .flatten()
            .collect();

        Ok(ParsedFile(file, headings, blocks))
    }
}
