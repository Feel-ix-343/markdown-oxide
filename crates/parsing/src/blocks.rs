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
    parent: Option<BlockSlot>,
    children: Option<Vec<BlockSlot>>,
    location: BlockFileLocation,
    outgoing: Vec<BlockSlot>,
    incoming: Option<Vec<BlockSlot>>,
}

/// Shared, mutable slot for a Block. This allows us to calculate complex recursive relationships between Blocks inside
/// the Block struct itself.
///
/// It acts as a deferred initialization so that we can construct recursive datastructures without infinite recursion.
///
/// Once the struct using this is constructed to a usable state, all atomic block slots should be *Set*. Reading
/// an atomic slot returns a result to reflect this.
#[derive(Clone)]
struct BlockSlot(Arc<RwLock<SlotState>>);
#[derive(Clone)]
enum SlotState {
    Empty,
    Set(Arc<Block>),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
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
            .field("incoming", &self.incoming)
            .finish()
    }
}

/// Block methods
impl Block {
    pub fn parent(&self) -> Option<anyhow::Result<Arc<Block>>> {
        self.parent.as_ref().map(|shared| shared.read())
    }

    pub fn children(&self) -> Option<anyhow::Result<Vec<Arc<Block>>>> {
        self.children
            .as_ref()
            .map(|children| children.iter().map(|child| child.read()).collect())
    }

    pub fn location(&self) -> &BlockFileLocation {
        &self.location
    }

    /// Debug if block is initialized, showing proper construction algorithm
    pub(crate) fn is_initialized(&self) -> bool {
        let Block {
            parent,
            children,
            location: _,
            outgoing,
            incoming,
        } = self;
        if let Some(parent) = parent {
            if !parent.is_initialized() {
                return false;
            }
        }

        if let Some(children) = children {
            if !children.iter().all(|child| child.is_initialized()) {
                return false;
            }
        }

        if !outgoing.iter().all(|out| out.is_initialized()) {
            return false;
        }

        if let Some(incoming) = incoming {
            if !incoming.iter().all(|inc| inc.is_initialized()) {
                return false;
            };
        };

        true
    }
}

/// Cx with cheap clone
///
/// Contains data structures used for constructing blocks
#[derive(Clone)]
pub(crate) struct BlockCx<'a> {
    location_id_map: Arc<LocationIDMap>,
    index_id_map: Arc<IndexIDMap>,
    outgoing_map: Arc<OutgoingMap>,
    incoming_map: Arc<IncomingMap>,
    root_dir: &'a Path,
    file_path: &'a Path,
}

/// An ID for a block based on its location in the filesystem.
///
/// Unlike IndexID, each block can have a LocationID and each LocationID can only belong to one block
///
/// Indexes by full file path and block location in file
///
/// This type should be cheap to construct and clone
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct LocationID(Arc<Path>, BlockFileLocation);

/// This will be the block's index id.
///
/// The first segment can either be the full path from the root_dir or the file name
///
/// The second segment is the block's given index
///
/// Only blocks with a specified index -- ^fja223 appended -- will have an IndexID
///
/// Multiple blocks may share an IndexID, though this is uncommon.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct IndexID(String);

/// Outgoing indexes; Indexes may not exist
struct OutgoingMap(HashMap<LocationID, HashSet<IndexID>>);

/// Locations for index; LocationID must exist
struct IncomingMap(HashMap<IndexID, HashSet<LocationID>>);

#[derive(Deref)]
struct LocationIDMap(HashMap<LocationID, BlockSlot>);

#[derive(Deref)]
struct IndexIDMap(HashMap<IndexID, BlockSlot>);

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
        parent: Option<BlockSlot>,
    ) -> anyhow::Result<(BlockSlot, Vec<Arc<Block>>)> {
        let location = list_block.location();

        let this_location_id = LocationID::for_block(list_block, cx.file_path.into());
        let outgoing = Self::outgoing(cx, &this_location_id)
            .context("Outgoing for list block")
            .unwrap();

        let index_ids = IndexID::for_block(list_block, cx.file_path, cx.root_dir);
        let incoming =
            Self::incoming(&index_ids, cx).map(|it| it.context("Incoming for list block").unwrap());

        match list_block.children() {
            None => {
                let block = Arc::new(Block {
                    parent,
                    children: None,
                    location,
                    outgoing,
                    incoming,
                });

                let shared_block = BlockSlot::new(block.clone());

                Self::set_slots_for_ids(cx, &this_location_id, index_ids, block.clone())
                    .context(format!("In most child block"))
                    .unwrap();

                Ok((shared_block, vec![block]))
            }
            Some(children) => {
                let uninitialized_this = BlockSlot::empty();

                let r = children.iter().try_fold(
                    (Vec::<BlockSlot>::new(), Vec::<Arc<Block>>::new()),
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
                            incoming,
                        });

                        Self::set_slots_for_ids(cx, &this_location_id, index_ids, block.clone())
                            .context(format!("In most parent block"))
                            .unwrap();

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
        let this_location_id = LocationID::for_block(paragraph_block, cx.file_path.into());
        let index_ids = IndexID::for_block(paragraph_block, cx.file_path, cx.root_dir);
        let outgoing = Self::outgoing(cx, &this_location_id)
            .context("Outgoing for paragraph block")
            .unwrap();
        let incoming = Self::incoming(&index_ids, cx)
            .map(|it| it.context("Incoming for paragraph block").unwrap());

        let block = Arc::new(Self {
            parent: None,
            children: None,
            location: BlockFileLocation::from_range(paragraph_block.range),
            outgoing,
            incoming,
        });

        Self::set_slots_for_ids(cx, &this_location_id, index_ids, block.clone())
            .context("For paragraph block")
            .unwrap();

        Ok(block)
    }

    fn outgoing(cx: &BlockCx<'_>, this_location_id: &LocationID) -> anyhow::Result<Vec<BlockSlot>> {
        let outgoing_index_ids = cx
            .outgoing_map
            .outgoing_for_location_id(this_location_id)
            .context("Getting outgoing ids for block")?;
        let outgoing = outgoing_index_ids
            .into_iter()
            .flat_map(|idx| cx.index_id_map.get(idx).cloned())
            .collect();
        Ok(outgoing)
    }

    fn incoming(
        index_ids: &Option<Vec<IndexID>>,
        cx: &BlockCx<'_>,
    ) -> Option<anyhow::Result<Vec<BlockSlot>>> {
        let incoming = index_ids.clone().map(|ids| {
            let incoming_location_ids = cx.incoming_map.incoming_for_index_ids(&ids);
            let incoming_slots = incoming_location_ids
                .into_iter()
                .map(|id| {
                    anyhow::Ok(
                        cx.location_id_map
                            .get(id)
                            .context("Getting slot for id from incoming ids")?
                            .clone(),
                    )
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(incoming_slots)
        });
        incoming
    }

    fn set_slots_for_ids(
        cx: &BlockCx,
        location_id: &LocationID,
        index_ids: Option<Vec<IndexID>>,
        block: Arc<Block>,
    ) -> anyhow::Result<()> {
        let _ = cx
            .location_id_map
            .set_block(location_id, block.clone())
            .context("In block set slots function")?;
        if let Some(index_ids) = index_ids {
            cx.index_id_map
                .set_block_given_index(&index_ids, block.clone())
                .context("In block set slots function")?;
        }
        Ok(())
    }
}

trait DocumentListBlockAdapter: Sized {
    fn children(&self) -> &Option<Vec<Self>>;
    fn location(&self) -> BlockFileLocation;
    fn list_block_index(&self) -> Option<&str>;
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
        BlockFileLocation::from_range(self.range)
    }

    fn link_refs(&self) -> impl Iterator<Item = &str> {
        self.content.link_refs()
    }

    fn list_block_index(&self) -> Option<&str> {
        self.content.index.as_ref().map(|it| it.as_ref())
    }
}

impl BlockSlot {
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

impl Debug for BlockSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slot_state = self.0.read().expect("Failed to read from RwLock");

        f.debug_struct("Atomic Block Slot")
            .field("State", &slot_state)
            .finish()
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

impl LocationIDMap {
    /// Constructs LocationIDMap for all locations, setting to empty block slots.
    fn empty_from_documents(documents: &Documents) -> Self {
        Self(
            documents
                .documents()
                .par_iter()
                .map(|(path, document)| {
                    document
                        .all_blocks()
                        .map(|block| {
                            let id = LocationID::for_block(&block, path.clone());
                            let empty_slot = BlockSlot::empty();
                            (id, empty_slot)
                        })
                        .collect::<Vec<_>>() // TODO: can we remove this?
                        .into_par_iter()
                })
                .flatten()
                .collect(),
        )
    }
}

impl IndexIDMap {
    fn empty_from_documents(documents: &Documents, root_dir: &Path) -> Self {
        Self(
            documents
                .documents()
                .into_par_iter()
                .map(|(path, document)| {
                    document
                        .all_blocks()
                        .flat_map(|block| {
                            let index_ids = IndexID::for_block(&block, path, root_dir)?;
                            let empty_slot = BlockSlot::empty();

                            Some((index_ids, empty_slot))
                        })
                        .map(|(ids, slot)| ids.into_iter().map(move |id| (id, slot.clone())))
                        .flatten()
                        .collect::<Vec<_>>()
                        .into_par_iter()
                })
                .flatten()
                .collect(),
        )
    }
}

// Behavior
impl IndexIDMap {
    /// Set block into index given that it has an index specified
    fn set_block_given_index(
        &self,
        indexes: &Vec<IndexID>,
        block: Arc<Block>,
    ) -> anyhow::Result<()> {
        let idx = &indexes[0];
        let slot = self.get(idx).ok_or(anyhow!(
            "Block index, {idx:?}, should exist in map if it is being set" // This is because we should have already parsed the block,
                                                                           // calculated its index(s), and set them
        ))?;
        // setting one index will set them all

        let _ = slot.initialize(block)?;
        Ok(())
    }
}

// Behavior
impl LocationIDMap {
    fn set_block(&self, location_id: &LocationID, block: Arc<Block>) -> anyhow::Result<()> {
        let slot = self.get(location_id).ok_or(anyhow!(
            "Block location should exist in map if it is being set"
        ))?;

        let _ = slot.initialize(block)?;
        Ok(())
    }
}

/// Trait representing DocBlock entity
///
/// Includes for example all of the DocBlock, BorrowedDocBlock and their enum members
/// ListBlock, ParagraphBlock ...
trait DocBlockAdapter {
    fn file_location(&self) -> BlockFileLocation;
    fn index(&self) -> Option<&str>;
    fn link_refs(&self) -> impl Iterator<Item = &str>;
}

impl LocationID {
    fn for_block(block: &impl DocBlockAdapter, file: Arc<Path>) -> Self {
        let location = block.file_location();
        let path = file.clone();
        Self(path, location)
    }
}

impl IndexID {
    fn for_block(
        block: &impl DocBlockAdapter,
        full_file_path: &Path,
        root_dir: &Path,
    ) -> Option<Vec<Self>> {
        let index = block.index()?;
        let file_name = full_file_path
            .file_stem()
            .expect("Failed to get file stem")
            .to_str()
            .expect("Failed to convert file stem to string");
        let string_path_from_root = diff_paths(full_file_path, root_dir)
            .expect("Paths should diff")
            .with_file_name(file_name)
            .to_string_lossy()
            .to_string();

        Some(
            vec![
                Self(format!("{string_path_from_root}#^{index}")),
                Self(format!("{file_name}#^{index}")),
            ]
            .into_iter()
            .unique()
            .collect(),
        )
    }

    fn for_block_links(block: &impl DocBlockAdapter) -> HashSet<Self> {
        block
            .link_refs()
            .map(|link| Self(link.to_string()))
            .collect()
    }
}

impl DocBlockAdapter for BorrowedDocBlock<'_> {
    fn file_location(&self) -> BlockFileLocation {
        BlockFileLocation::from_range(self.range())
    }

    fn index(&self) -> Option<&str> {
        self.content().index.as_ref().map(|idx| idx.as_ref())
    }

    fn link_refs(&self) -> impl Iterator<Item = &str> {
        self.content().link_refs()
    }
}

impl<T: DocumentListBlockAdapter> DocBlockAdapter for T {
    fn link_refs(&self) -> impl Iterator<Item = &str> {
        self.link_refs()
    }
    fn index(&self) -> Option<&str> {
        self.list_block_index()
    }
    fn file_location(&self) -> BlockFileLocation {
        self.location()
    }
}

impl DocBlockAdapter for DocParagraphBlock {
    fn index(&self) -> Option<&str> {
        self.content.index.as_ref().map(|it| it.as_ref())
    }
    fn file_location(&self) -> BlockFileLocation {
        BlockFileLocation::from_range(self.range)
    }
    fn link_refs(&self) -> impl Iterator<Item = &str> {
        self.content.link_refs()
    }
}

impl OutgoingMap {
    fn from_documents(documents: &Documents) -> Self {
        Self(
            documents
                .documents()
                .par_iter()
                .map(|(path, document)| {
                    document
                        .all_blocks()
                        .map(|block| {
                            let this_location_id = LocationID::for_block(&block, path.clone());

                            let outgoing_index_ids = IndexID::for_block_links(&block);

                            (this_location_id, outgoing_index_ids)
                        })
                        .collect::<Vec<_>>()
                        .into_par_iter()
                })
                .flatten()
                .collect(),
        )
    }
}

impl IncomingMap {
    fn from_outgoing_map(map: &OutgoingMap) -> Self {
        let incoming_map = map
            .0
            .iter()
            .map(|(location_id, indexes)| {
                indexes
                    .iter()
                    .map(move |index| (index.clone(), location_id.clone()))
            })
            .flatten()
            .into_grouping_map()
            .collect();

        Self(incoming_map)
    }
}

// Behavior
impl OutgoingMap {
    /// Not all indexes must exist; they were derived from the text of links
    fn outgoing_for_location_id(
        &self,
        location_id: &LocationID,
    ) -> anyhow::Result<&HashSet<IndexID>> {
        self.0.get(location_id).ok_or(anyhow!(
            "Location ID {location_id:?} should exist in map if being accessed"
        ))
    }
}

impl IncomingMap {
    /// Location ids corresponding to blocks that were linked will appear here. If there are no link, this will
    /// be an empty collection
    fn incoming_for_index_ids(&self, index_ids: &Vec<IndexID>) -> HashSet<&LocationID> {
        index_ids
            .iter()
            .flat_map(|id| self.0.get(id).map(|it| it.iter()))
            .flatten()
            .collect()
    }
}

/// Construction
impl BlockCx<'_> {
    pub(crate) fn new<'a>(
        documents: &'a Documents,
        root_dir: &'a Path,
    ) -> impl Fn(&'a Path) -> BlockCx<'a> {
        let location_id_map = Arc::new(LocationIDMap::empty_from_documents(documents));
        let index_id_map = Arc::new(IndexIDMap::empty_from_documents(documents, root_dir));
        let outgoing_map = Arc::new(OutgoingMap::from_documents(documents));
        let incoming_map = Arc::new(IncomingMap::from_outgoing_map(&outgoing_map));

        move |path: &'a Path| BlockCx {
            location_id_map: location_id_map.clone(),
            index_id_map: index_id_map.clone(),
            outgoing_map: outgoing_map.clone(),
            incoming_map: incoming_map.clone(),
            root_dir,
            file_path: path,
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

    use super::{DocBlockAdapter, DocumentListBlockAdapter, IndexID};

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

        fn list_block_index(&self) -> Option<&str> {
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

        fn list_block_index(&self) -> Option<&str> {
            None
        }
    }

    struct MockDocBlock;
    impl DocBlockAdapter for MockDocBlock {
        fn index(&self) -> Option<&str> {
            Some("12345")
        }

        fn file_location(&self) -> BlockFileLocation {
            todo!()
        }

        fn link_refs(&self) -> impl Iterator<Item = &str> {
            std::iter::empty()
        }
    }

    #[test]
    fn test_index_id() {
        let block = MockDocBlock;
        let indexes = IndexID::for_block(
            &block,
            Path::new("/home/felix/notes/test.md"),
            Path::new("/home/felix/notes"),
        );
        assert_eq!(
            Some(vec![
                IndexID("test#^12345".into()),
                // IndexID("test#^12345".into())
            ]),
            indexes
        )
    }
}
