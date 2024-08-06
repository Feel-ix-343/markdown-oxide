use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use std::fmt::Debug;

use anyhow::anyhow;
use derive_deref::Deref;
use itertools::Itertools;
use tree_sitter::Range;

use crate::document::{DocBlock, DocListBlock, DocParagraphBlock, DocSection, Document, Node};

#[derive(Deref, Debug)]
pub(crate) struct Blocks(Vec<Arc<Block>>);

/// All useful data regarding a block: hover, querying, go_to_definition, ...
#[derive()]
pub(crate) struct Block {
    parent: Option<AtomicBlockSlot>,
    children: Option<Vec<AtomicBlockSlot>>,
    location: BlockFileLocation,
    // raw_content: Arc<str>,
}

/// Shared, mutable slot for a Block. This allows us to calculate complex recursive relationships between Blocks inside
/// the Block struct itself.
///
/// It acts as a deferred initialization so that we can construct recursive datastructures without infinite recursion.
///
/// Once the struct using this is constructed to a usable state, all atomic block slots should be *Set*. Reading
/// an atomic slot returns a result to reflect this.
#[derive(Debug, Clone)]
struct AtomicBlockSlot(Arc<RwLock<SlotState>>);
#[derive(Clone)]
enum SlotState {
    Empty,
    Set(Arc<Block>),
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub(crate) enum BlockFileLocation {
    Line(Line),
    Lines(Lines),
}

pub(crate) type Line = usize;
pub(crate) type Lines = std::ops::Range<Line>;

impl Blocks {
    pub(crate) fn new(doc: &Document) -> anyhow::Result<Self> {
        let list_blocks = doc.block_containers();
        let blocks: anyhow::Result<Vec<_>> = list_blocks
            .into_iter()
            .map(|it| match it {
                DocBlock::ParagraphBlock(block) => {
                    let concrete = Arc::new(Block::from_paragraph(block));
                    anyhow::Ok(vec![concrete])
                }
                DocBlock::ListBlock(list_block) => {
                    let concrete = Block::new(list_block)?;
                    anyhow::Ok(concrete)
                }
            })
            .flatten_ok()
            .collect();

        Ok(Self(blocks?))
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field("location", &self.location())
            .field("parent", &self.parent)
            .field("children", &self.children)
            .finish()
    }
}

/// Block methods
impl Block {
    fn parent(&self) -> Option<anyhow::Result<Arc<Block>>> {
        self.parent.as_ref().map(|shared| shared.read())
    }

    fn children(&self) -> Option<anyhow::Result<Vec<Arc<Block>>>> {
        self.children
            .as_ref()
            .map(|children| children.iter().map(|child| child.read()).collect())
    }

    fn location(&self) -> &BlockFileLocation {
        &self.location
    }

    fn is_initialized(&self) -> bool {
        match (&self.children, &self.parent) {
            (Some(children), Some(parent)) => {
                children.iter().all(|child| child.is_initialized()) && parent.is_initialized()
            }
            _ => true,
        }
    }
}

/// Map used for constructing related blocks in Block struct.
///
/// All IDs will be set to an Empty slot from iterating over DocumentBlocks
struct BlockIDMap(HashMap<Arc<Path>, HashMap<BlockId, AtomicBlockSlot>>);
/// Path and Index, excluding the ^ in index
struct BlockId(Arc<Path>, Arc<str>);

/// Block construction
impl Block {
    fn new(list_block: &impl DocumentListBlockAdapter) -> anyhow::Result<Vec<Arc<Self>>> {
        let blocks = Self::recurse_list_block(list_block, None)?.1;

        Ok(blocks)
    }

    fn recurse_list_block(
        list_block: &impl DocumentListBlockAdapter,
        parent: Option<AtomicBlockSlot>,
    ) -> anyhow::Result<(AtomicBlockSlot, Vec<Arc<Block>>)> {
        let location = list_block.location();
        match list_block.children() {
            None => {
                let block = Arc::new(Block {
                    parent,
                    children: None,
                    location,
                });

                let shared_block = AtomicBlockSlot::new(block.clone());

                Ok((shared_block, vec![block]))
            }
            Some(children) => {
                let uninitialized_this = AtomicBlockSlot::empty();

                let r = children.iter().try_fold(
                    (Vec::<AtomicBlockSlot>::new(), Vec::<Arc<Block>>::new()),
                    |(mut children, mut all), child| {
                        Self::recurse_list_block(child, Some(uninitialized_this.clone())).map(
                            |(child, acc)| {
                                children.push(child);
                                all.extend(acc);

                                (children, all)
                            },
                        )
                    },
                );

                match r {
                    Ok((children, acc)) => {
                        let block = Arc::new(Block {
                            parent,
                            children: Some(children),
                            location,
                        });
                        let initialized = uninitialized_this.initialize(block.clone())?;

                        let mut all = vec![block];
                        all.extend(acc);

                        Ok((initialized, all))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn from_paragraph(paragraph_block: &DocParagraphBlock) -> Self {
        Self {
            parent: None,
            children: None,
            location: BlockFileLocation::from_range(paragraph_block.range),
        }
    }
}

impl BlockId {
    fn from_block(block: &DocBlock) -> Option<Self> {
        let index = block.
    }
}

trait DocumentListBlockAdapter: Sized {
    fn children(&self) -> &Option<Vec<Self>>;
    fn location(&self) -> BlockFileLocation;
}

impl BlockFileLocation {
    fn from_range(range: Range) -> BlockFileLocation {
        if range.start_point.row + 1 == range.end_point.row {
            BlockFileLocation::Line(range.start_point.row)
        } else {
            BlockFileLocation::Lines(range.start_point.row..range.end_point.row)
        }
    }
}

impl DocumentListBlockAdapter for DocListBlock {
    fn children(&self) -> &Option<Vec<Self>> {
        &self.children
    }
    fn location(&self) -> BlockFileLocation {
        BlockFileLocation::Line(self.range.start_point.row)
    }
}

impl AtomicBlockSlot {
    fn empty() -> Self {
        Self(Arc::new(RwLock::new(SlotState::Empty)))
    }

    fn initialize(&self, block: Arc<Block>) -> anyhow::Result<Self> {
        let mut write = self
            .0
            .write()
            .or(Err(anyhow!("Failed to read from lock when I shuold have")))?;
        *write = SlotState::Set(block);

        Ok(self.clone())
    }

    fn new(block: Arc<Block>) -> Self {
        Self(Arc::new(RwLock::new(SlotState::Set(block))))
    }

    fn is_initialized(&self) -> bool {
        match *self.0.read().expect("Broken RwLock") {
            SlotState::Empty => false,
            SlotState::Set(_) => true,
        }
    }

    fn read(&self) -> anyhow::Result<Arc<Block>> {
        let read = self
            .0
            .read()
            .map_err(|_| anyhow!("Failed to read from RwLock"))?;
        let block = match *read {
            SlotState::Empty => return Err(anyhow!("Block not initialized when it should be")),
            SlotState::Set(ref block) => block.clone(),
        };
        Ok(block)
    }
}

impl Debug for SlotState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotState::Empty => f.write_str("Empty"),
            SlotState::Set(block) => f
                .debug_struct("Initialized")
                .field("Location", &block.location)
                .finish(),
        }
    }
}

#[cfg(test)]
mod blocks_tests {
    use std::{
        path::{Path, PathBuf},
        sync::Arc,
    };

    use itertools::Itertools;

    use crate::blocks::{Block, BlockFileLocation};

    use super::DocumentListBlockAdapter;

    const LOCATION: BlockFileLocation = BlockFileLocation::Line(0);

    #[derive(Clone)]
    struct MockListBlockNoRange(Option<Vec<MockListBlockNoRange>>);
    impl DocumentListBlockAdapter for MockListBlockNoRange {
        fn location(&self) -> BlockFileLocation {
            LOCATION
        }

        fn children(&self) -> &Option<Vec<Self>> {
            &self.0
        }
    }

    struct MockListBlockRange(usize, Option<Vec<MockListBlockRange>>);
    impl DocumentListBlockAdapter for MockListBlockRange {
        fn location(&self) -> BlockFileLocation {
            BlockFileLocation::Line(self.0)
        }
        fn children(&self) -> &Option<Vec<Self>> {
            &self.1
        }
    }

    #[test]
    fn block_construction() {
        let list_block = MockListBlockRange(
            0,
            Some(vec![
                MockListBlockRange(1, None),
                MockListBlockRange(2, Some(vec![MockListBlockRange(3, None)])),
            ]),
        );

        let output = Block::new(&list_block);
        assert!(output.is_ok());
    }
}
