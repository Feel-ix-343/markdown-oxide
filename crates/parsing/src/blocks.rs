use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use std::fmt::Debug;

use anyhow::{anyhow, Context};
use derive_deref::Deref;
use itertools::Itertools;
use pathdiff::diff_paths;
use rayon::prelude::*;
use tree_sitter::Range;

use crate::{
    document::{
        BorrowedDocBlock, DocBlock, DocListBlock, DocParagraphBlock, DocSection, Document, Node,
    },
    documents::Documents,
};

#[derive(Deref, Debug)]
pub(crate) struct Blocks(Vec<Arc<Block>>);

/// All useful data regarding a block: hover, querying, go_to_definition, ...
#[derive()]
pub(crate) struct Block {
    parent: Option<AtomicBlockSlot>,
    children: Option<Vec<AtomicBlockSlot>>,
    location: BlockFileLocation,
    outgoing: Vec<AtomicBlockSlot>,
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
    pub(crate) fn new(cx: BlockCx, doc: &Document) -> anyhow::Result<Self> {
        let doc_blocks = doc.top_level_doc_blocks();
        let blocks: anyhow::Result<Vec<_>> = doc_blocks
            .map(|it| match it {
                DocBlock::ParagraphBlock(block) => {
                    let concrete = Block::from_paragraph(block, &cx)?;
                    Ok(vec![concrete])
                }
                DocBlock::ListBlock(list_block) => {
                    let concrete = Block::new(list_block, &cx)?;
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
            .field("outgoing", &self.outgoing)
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
#[derive(Deref)]
struct PersistentBlockIdMap(HashMap<Arc<Path>, HashMap<Arc<BlockId>, AtomicBlockSlot>>);
#[derive(Debug, Hash, PartialEq, Eq)]
struct BlockId(String);

#[derive(Clone, Deref)]
struct BlockIdMap(HashMap<Arc<BlockId>, AtomicBlockSlot>);

#[derive(Clone)]
/// Cx with Cheap clone
pub(crate) struct BlockCx<'a> {
    block_id_map: Arc<BlockIdMap>,
    persistent_block_id_map: Arc<PersistentBlockIdMap>,
    path_from_root: Arc<Path>,
    file_path: &'a Path,
}

/// Block construction
impl Block {
    fn new(
        list_block: &impl DocumentListBlockAdapter,
        cx: &BlockCx,
    ) -> anyhow::Result<Vec<Arc<Self>>> {
        let blocks = Self::recurse_list_block(list_block, cx, None)?.1;

        Ok(blocks)
    }

    fn recurse_list_block(
        list_block: &impl DocumentListBlockAdapter,
        cx: &BlockCx,
        parent: Option<AtomicBlockSlot>,
    ) -> anyhow::Result<(AtomicBlockSlot, Vec<Arc<Block>>)> {
        let BlockCx {
            block_id_map,
            path_from_root,
            file_path,
            persistent_block_id_map,
        } = cx.clone();

        let location = list_block.location();

        let outgoing = Self::outgoing(&block_id_map, list_block);

        match list_block.children() {
            None => {
                let block = Arc::new(Block {
                    parent,
                    children: None,
                    location,
                    outgoing,
                });

                let shared_block = AtomicBlockSlot::new(block.clone());

                persistent_block_id_map
                    .set_block(list_block, block.clone(), cx.clone())
                    .context(format!(
                        "Setting most child block in {:?}",
                        list_block.list_block_index()
                    ))?;

                Ok((shared_block, vec![block]))
            }
            Some(children) => {
                let uninitialized_this = AtomicBlockSlot::empty();

                let r = children.iter().try_fold(
                    (Vec::<AtomicBlockSlot>::new(), Vec::<Arc<Block>>::new()),
                    |(mut children, mut all), child| {
                        Self::recurse_list_block(child, cx, Some(uninitialized_this.clone())).map(
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
                            outgoing,
                        });

                        persistent_block_id_map
                            .set_block(list_block, block.clone(), cx.clone())
                            .context(format!(
                                "Setting parent block in {:?}",
                                list_block.list_block_index()
                            ))?;

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

    fn from_paragraph(
        paragraph_block: &DocParagraphBlock,
        cx: &BlockCx,
    ) -> anyhow::Result<Arc<Self>> {
        let BlockCx {
            block_id_map,
            persistent_block_id_map,
            ..
        } = cx;

        let block = Arc::new(Self {
            parent: None,
            children: None,
            location: BlockFileLocation::from_range(paragraph_block.range),
            outgoing: Self::outgoing(&block_id_map, paragraph_block),
        });

        persistent_block_id_map
            .set_block(paragraph_block, block.clone(), cx.clone())
            .context("Setting parent block")?;

        Ok(block)
    }
}

/// Note that these do not *have* to be block links
trait HasOutgoingLinks {
    fn outgoing_links(&self) -> impl Iterator<Item = &str>;
}

/// Related block construction: incoming, outgoing
impl Block {
    /// Outgoing links; only resolved ones.
    fn outgoing(
        block_id_map: &BlockIdMap,
        has_outgoing: &impl HasOutgoingLinks,
    ) -> Vec<AtomicBlockSlot> {
        let outgoing_links_texts = has_outgoing.outgoing_links();
        outgoing_links_texts
            .flat_map(|text| {
                let id = BlockId::from_string(text.to_string());
                let block = block_id_map.get(&id);
                block.cloned()
            })
            .collect()
    }
}

impl HasOutgoingLinks for DocParagraphBlock {
    fn outgoing_links(&self) -> impl Iterator<Item = &str> {
        self.content.link_refs()
    }
}

impl<T: DocumentListBlockAdapter> HasOutgoingLinks for T {
    fn outgoing_links(&self) -> impl Iterator<Item = &str> {
        self.link_refs()
    }
}

trait HasIndex {
    fn index(&self) -> Option<Arc<str>>;
}

impl BlockId {
    fn from_block(path_from_root: &Path, block: &dyn HasIndex) -> Option<Vec<Self>> {
        let path_as_string = path_from_root
            .to_str()
            .expect("Failed to convert path to string");
        let block_index = block.index()?;
        let file_name = path_from_root
            .file_stem()
            .expect("Failed to get file stem")
            .to_str()
            .expect("Failed to convert file stem to string");
        Some(vec![
            // Self(format!("{path_as_string}#^{block_index}")),
            Self(format!("{file_name}#^{block_index}")),
        ])
    }

    fn from_string(string: String) -> Self {
        Self(string)
    }
}

impl<T: DocumentListBlockAdapter> HasIndex for T {
    fn index(&self) -> Option<Arc<str>> {
        self.list_block_index()
    }
}

impl HasIndex for DocParagraphBlock {
    fn index(&self) -> Option<Arc<str>> {
        self.content.index.clone()
    }
}

impl HasIndex for DocBlock {
    fn index(&self) -> Option<Arc<str>> {
        self.doc_index()
    }
}

impl HasIndex for BorrowedDocBlock<'_> {
    fn index(&self) -> Option<Arc<str>> {
        match self {
            Self::ParagraphBlock(p) => p.index(),
            Self::ListBlock(l) => l.index(),
        }
    }
}

/// Behavior
impl PersistentBlockIdMap {
    /// set the block for a block slot. This will *saturate/set* referring blocks *related* style fields.
    fn set_block(
        &self,
        has_index: &impl HasIndex,
        block: Arc<Block>,
        cx: BlockCx,
    ) -> anyhow::Result<()> {
        if let Some(id) = BlockId::from_block(&cx.path_from_root, has_index) {
            let first = &id[0];
            let slot = self
                .get(cx.file_path)
                .and_then(|map| map.get(first))
                .ok_or(anyhow!(format!(
                "Block index should exist in map if we are iterating through it; index: {first:?}"
            )))?;

            match slot.initialize(block.clone()) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        }
    }
}

impl PersistentBlockIdMap {
    fn empty_from_documents(documents: &Documents) -> Self {
        Self(
            documents
                .documents()
                .par_iter()
                .map(|(path, document)| {
                    let m: HashMap<_, _> = document
                        .all_blocks()
                        .flat_map(|it| {
                            let empty = AtomicBlockSlot::empty(); // keep the same block for all id's.

                            BlockId::from_block(path, &it).map(move |it| {
                                it.into_iter().map(move |it| (Arc::new(it), empty.clone()))
                            })
                        })
                        .flatten()
                        .collect();

                    (path.clone(), m)
                })
                .collect(),
        )
    }
}

impl BlockIdMap {
    // TODO: Handle duplicate ids in different files.
    fn from_persistant(map: Arc<PersistentBlockIdMap>) -> Self {
        Self(
            map.0
                .par_iter()
                .map(|(_, file_map)| {
                    file_map
                        .into_par_iter()
                        .map(|(id, slot)| (id.clone(), slot.clone()))
                })
                .flatten()
                .collect(),
        )
    }
}

trait DocumentListBlockAdapter: Sized {
    fn children(&self) -> &Option<Vec<Self>>;
    fn location(&self) -> BlockFileLocation;
    fn list_block_index(&self) -> Option<Arc<str>>;
    fn link_refs(&self) -> impl Iterator<Item = &str>;
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
        BlockFileLocation::Line(self.range.start_point.row + 1) // TODO: Should this be +1? I can't remember
    }

    fn link_refs(&self) -> impl Iterator<Item = &str> {
        self.content.link_refs()
    }

    fn list_block_index(&self) -> Option<Arc<str>> {
        self.content.index.clone()
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

impl BlockCx<'_> {
    pub(crate) fn new<'a>(
        documents: &'a Documents,
        root_dir: &'a Path,
    ) -> impl Fn(&'a Path) -> BlockCx<'a> {
        let persistent_block_id_map =
            Arc::new(PersistentBlockIdMap::empty_from_documents(documents));
        let block_id_map = Arc::new(BlockIdMap::from_persistant(persistent_block_id_map.clone()));

        move |path: &'a Path| {
            let mut path_from_root = diff_paths(root_dir, path).expect("Failed to diff paths");
            path_from_root.push(path.file_name().expect("path should be file with name"));
            BlockCx {
                block_id_map: block_id_map.clone(),
                persistent_block_id_map: persistent_block_id_map.clone(),
                path_from_root: Arc::from(path_from_root.as_path()),
                file_path: path,
            }
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

        fn list_block_index(&self) -> Option<Arc<str>> {
            None
        }

        fn link_refs(&self) -> impl Iterator<Item = &str> {
            std::iter::empty()
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

        fn link_refs(&self) -> impl Iterator<Item = &str> {
            std::iter::empty()
        }

        fn list_block_index(&self) -> Option<Arc<str>> {
            None
        }
    }
}
