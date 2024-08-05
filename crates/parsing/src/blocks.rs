use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::anyhow;
use derive_deref::Deref;
use itertools::Itertools;
use tree_sitter::Range;

use crate::document::{BlockContainer, Document, ListBlock, Node, ParagraphBlock, Section};

#[derive(Deref, Debug)]
pub(crate) struct Blocks(Vec<Arc<dyn Block>>);

/// All useful data regarding a block: hover, querying, go_to_definition, ...
pub(crate) trait Block: Send + Sync {
    fn parent(&self) -> Option<anyhow::Result<Arc<dyn Block>>>;
    fn children(&self) -> Option<anyhow::Result<Vec<Arc<dyn Block>>>>;
    fn location(&self) -> &BlockFileLocation;
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum BlockFileLocation {
    Line(Line),
    Lines(Lines),
}

type Line = usize;
type Lines = std::ops::Range<Line>;

use std::fmt::Debug;

impl Debug for dyn Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            // .field("parent", &self.parent())
            .field("children", &self.children())
            .field("location", &self.location())
            .finish()
    }
}

impl Blocks {
    pub(crate) fn new(doc: &Document) -> Self {
        let list_blocks = doc.block_containers();
        let blocks = list_blocks
            .into_iter()
            .map(|it| match it {
                BlockContainer::ParagraphBlock(block) => {
                    let concrete: Arc<dyn Block> = Arc::new(ConcreteBlock::from_paragraph(block));
                    vec![concrete]
                }
                BlockContainer::ListBlock(list_block) => {
                    let concrete = ConcreteBlock::from_list_block(list_block);
                    concrete
                        .into_iter()
                        .map(|it| it as Arc<dyn Block>)
                        .collect()
                }
            })
            .flatten()
            .collect();

        Self(blocks)
    }
}

/// All useful data regarding a block: hover, querying, go_to_definition, ...
#[derive(Debug)]
struct ConcreteBlock {
    parent: Option<Arc<RwLock<Option<Arc<ConcreteBlock>>>>>,
    children: Option<Vec<Arc<RwLock<Option<Arc<ConcreteBlock>>>>>>,
    location: Arc<BlockFileLocation>,
    // raw_content: Arc<str>,
}

impl Block for ConcreteBlock {
    fn parent(&self) -> Option<anyhow::Result<Arc<dyn Block>>> {
        match &self.parent {
            Some(arcmut) => {
                let read = arcmut.read();
                let Ok(option) = read.as_ref() else {
                    return Some(Err(anyhow!("Failed to read parent from RwLock")));
                };
                let Some(arc) = option.as_ref() else {
                    return Some(Err(anyhow!("Failed to read from block option")));
                };

                Some(Ok(arc.clone()))
            }
            None => None,
        }
    }

    fn children(&self) -> Option<anyhow::Result<Vec<Arc<dyn Block>>>> {
        match &self.children {
            Some(children) => Some({
                children
                    .iter()
                    .map(|child| {
                        let read = child.read();
                        let arc = read
                            .as_ref()
                            .or(Err(anyhow!("Failed to read children from RwLock")))?
                            .as_ref()
                            .ok_or(anyhow!("Failed to read from block option"))?;

                        Ok(arc.clone() as Arc<dyn Block>)
                    })
                    .collect()
            }),
            _ => None,
        }
    }

    fn location(&self) -> &BlockFileLocation {
        &self.location
    }
}

impl ConcreteBlock {
    fn from_list_block(list_block: &impl DocumentListBlock) -> Vec<Arc<Self>> {
        let joined = Joined::from_list_block(list_block);
        ConcreteBlock::from_joined(&joined)
    }

    fn from_paragraph(paragraph_block: &ParagraphBlock) -> Self {
        Self {
            parent: None,
            children: None,
            location: Arc::new(BlockFileLocation::from_range(paragraph_block.range)),
        }
    }

    fn from_joined(joined: &Joined) -> Vec<Arc<ConcreteBlock>> {
        let m = joined.iter().fold(
            HashMap::<Arc<BlockFileLocation>, Arc<RwLock<Option<Arc<ConcreteBlock>>>>>::new(),
            |mut acc, (location, (parent, children))| {
                let block = ConcreteBlock {
                    parent: parent.1.as_ref().map(|parent| {
                        let parent_mutex = acc
                            .entry(parent.0.clone())
                            .or_insert(Arc::new(RwLock::new(None)));
                        parent_mutex.clone()
                    }),
                    children: children.1.as_ref().map(|children| {
                        children
                            .iter()
                            .map(|child| {
                                let child_mutex = acc
                                    .entry(child.0.clone())
                                    .or_insert(Arc::new(RwLock::new(None)));
                                child_mutex.clone()
                            })
                            .collect()
                    }),
                    location: location.clone(),
                };

                match acc.get_mut(location) {
                    Some(this_mutex) => {
                        let mut mutex_guard = this_mutex.write().expect("Broken Mutex");
                        *mutex_guard = Some(Arc::new(block));
                    }
                    None => {
                        acc.insert(
                            location.clone(),
                            Arc::new(RwLock::new(Some(Arc::new(block)))),
                        );
                    }
                };

                acc
            },
        );

        m.values()
            .map(|it| {
                let guard = it.read().expect("Broken Mutex");
                let block = guard.as_ref().unwrap();
                block.clone()
            })
            .collect()
    }
}

// structs to construct the block data structure.
// First we collect all blocks by arc with references
// to children and parent -- seperately. Then we join these together to create block

#[derive(Debug, PartialEq, Eq)]
struct BlockWithParent(Arc<BlockFileLocation>, Option<Arc<BlockWithParent>>);

#[derive(Debug, PartialEq, Eq)]
struct BlockWithChildren(Arc<BlockFileLocation>, Option<Vec<Arc<BlockWithChildren>>>);

//
// Block UI Location serves as the index in the join

#[derive(Deref, Debug)]
struct ChildrenMap(HashMap<Arc<BlockFileLocation>, Arc<BlockWithChildren>>);

#[derive(Deref, Debug)]
struct ParentMap(HashMap<Arc<BlockFileLocation>, Arc<BlockWithParent>>);

#[derive(Deref, Debug)]
struct Joined(HashMap<Arc<BlockFileLocation>, (Arc<BlockWithParent>, Arc<BlockWithChildren>)>);

impl Joined {
    fn from_list_block(list_block: &impl DocumentListBlock) -> Self {
        let parent_map = ParentMap::from_list_block(list_block);
        let children_map = ChildrenMap::from_list_block(list_block);
        Self::from_maps(&parent_map, &children_map)
    }

    fn from_maps(parent_map: &ParentMap, children_map: &ChildrenMap) -> Self {
        let zipped = parent_map
            .iter()
            .flat_map(|(key, value)| {
                let with_children = children_map.get(key)?;
                Some((key.clone(), (value.clone(), with_children.clone())))
            })
            .collect();

        Joined(zipped)
    }
}

trait DocumentListBlock: Sized {
    fn children(&self) -> &Option<Vec<Self>>;
    fn location(&self) -> BlockFileLocation;
}

impl ChildrenMap {
    fn from_list_block(list_block: &impl DocumentListBlock) -> Self {
        let children_blocks = BlockWithChildren::from_list_block(list_block);
        Self(
            children_blocks
                .into_iter()
                .map(|it| (it.0.clone(), it))
                .collect(),
        )
    }
}

impl BlockWithChildren {
    fn from_list_block(list_block: &impl DocumentListBlock) -> Vec<Arc<BlockWithChildren>> {
        let (_, all) = Self::recurse_list_block(list_block);
        all.collect()
    }

    fn recurse_list_block<'a>(
        list_block: &impl DocumentListBlock,
    ) -> (
        Arc<BlockWithChildren>,
        Box<dyn Iterator<Item = Arc<BlockWithChildren>> + 'a>,
    ) {
        match (list_block.location(), list_block.children()) {
            (location, None) => {
                let arc = Arc::new(BlockWithChildren(location.into(), None));

                (arc.clone(), Box::new(std::iter::once(arc)))
            }
            (location, Some(children)) => {
                let (children, children_acc) = children.iter().fold(
                    (
                        Vec::<Arc<BlockWithChildren>>::new(),
                        Box::new(std::iter::empty())
                            as Box<dyn Iterator<Item = Arc<BlockWithChildren>>>,
                    ),
                    |(mut children, acc), list_child| {
                        let (child, list) = BlockWithChildren::recurse_list_block(list_child);
                        children.push(child);
                        let acc = Box::new(acc.chain(list));
                        (children, acc)
                    },
                );
                let this = Arc::new(BlockWithChildren(location.into(), Some(children)));
                (
                    this.clone(),
                    Box::new(std::iter::once(this).chain(children_acc)),
                )
            }
        }
    }
}

impl ParentMap {
    fn from_list_block(list_block: &impl DocumentListBlock) -> Self {
        Self(
            BlockWithParent::from_list_block(list_block)
                .into_iter()
                .map(|it| (it.0.clone(), it))
                .collect(),
        )
    }
}

impl BlockWithParent {
    fn from_list_block(list_block: &impl DocumentListBlock) -> Vec<Arc<BlockWithParent>> {
        Self::recurse_list_block(list_block, None).collect()
    }

    fn recurse_list_block<'a>(
        list_block: &'a impl DocumentListBlock,
        parent: Option<Arc<BlockWithParent>>,
    ) -> Box<dyn Iterator<Item = Arc<BlockWithParent>> + 'a> {
        match (list_block.location(), list_block.children()) {
            (location, None) => Box::new(std::iter::once(Arc::new(BlockWithParent(
                location.into(),
                parent,
            )))),
            (location, Some(children)) => {
                let this = Arc::new(BlockWithParent(location.into(), parent));

                let once = std::iter::once(this.clone());

                let children = children
                    .into_iter()
                    .flat_map(move |child| Self::recurse_list_block(child, Some(this.clone())));

                Box::new(once.chain(children))
            }
        }
    }
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

impl DocumentListBlock for ListBlock {
    fn children(&self) -> &Option<Vec<Self>> {
        &self.children
    }
    fn location(&self) -> BlockFileLocation {
        BlockFileLocation::Line(self.range.start_point.row)
    }
}

#[cfg(test)]
mod blocks_tests {
    use std::{
        path::{Path, PathBuf},
        sync::Arc,
    };

    use itertools::Itertools;

    use crate::blocks::{BlockFileLocation, ConcreteBlock};

    use super::{BlockWithChildren, BlockWithParent, DocumentListBlock, Joined};

    const LOCATION: BlockFileLocation = BlockFileLocation::Line(0);

    #[derive(Clone)]
    struct MockListBlockNoRange(Option<Vec<MockListBlockNoRange>>);
    impl DocumentListBlock for MockListBlockNoRange {
        fn location(&self) -> BlockFileLocation {
            LOCATION
        }

        fn children(&self) -> &Option<Vec<Self>> {
            &self.0
        }
    }

    #[test]
    fn test_blocks_with_children_for_list_block() {
        let list_block = MockListBlockNoRange(Some(vec![
            MockListBlockNoRange(None),
            MockListBlockNoRange(Some(vec![MockListBlockNoRange(None)])),
        ]));

        let output = BlockWithChildren::from_list_block(&list_block);

        let location: Arc<_> = LOCATION.into();
        let b1 = Arc::new(BlockWithChildren(location.clone(), None));
        let b2 = Arc::new(BlockWithChildren(location.clone(), None));
        let b3 = Arc::new(BlockWithChildren(location.clone(), Some(vec![b2.clone()])));
        let last = Arc::new(BlockWithChildren(
            location.clone(),
            Some(vec![b1.clone(), b3.clone()]),
        ));
        let expected = vec![last.clone(), b1.clone(), b3.clone(), b2.clone()];

        assert_eq!(output, expected);

        // Sharing memory correctly
        assert!(Arc::ptr_eq(
            &output[1],
            &output[0].as_ref().1.as_ref().unwrap()[0]
        ))
    }

    #[test]
    fn test_block_with_parent_list_block() {
        let list_block = MockListBlockNoRange(Some(vec![
            MockListBlockNoRange(None),
            MockListBlockNoRange(Some(vec![MockListBlockNoRange(None)])),
        ]));

        let output = BlockWithParent::from_list_block(&list_block);

        assert!(Arc::ptr_eq(
            &output[0],
            output[1].as_ref().1.as_ref().unwrap()
        ));
        assert!(Arc::ptr_eq(
            &output[0],
            output[2].as_ref().1.as_ref().unwrap()
        ));
        assert!(Arc::ptr_eq(
            &output[2],
            output[3].as_ref().1.as_ref().unwrap()
        ));
    }

    struct MockListBlockRange(usize, Option<Vec<MockListBlockRange>>);
    impl DocumentListBlock for MockListBlockRange {
        fn location(&self) -> BlockFileLocation {
            BlockFileLocation::Line(self.0)
        }
        fn children(&self) -> &Option<Vec<Self>> {
            &self.1
        }
    }

    #[test]
    fn test_joined_from_list_block() {
        let list_block = MockListBlockRange(
            0,
            Some(vec![
                MockListBlockRange(1, None),
                MockListBlockRange(2, Some(vec![MockListBlockRange(3, None)])),
            ]),
        );

        let output = Joined::from_list_block(&list_block);
        assert!(output.len() == 4);
    }
}
