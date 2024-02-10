use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    iter,
    ops::{Deref, DerefMut, Range},
    path::{Path, PathBuf},
};

use itertools::Itertools;
use once_cell::sync::Lazy;
use pathdiff::diff_paths;
use rayon::prelude::*;
use regex::Regex;
use ropey::Rope;
use tower_lsp::lsp_types::Position;
use walkdir::WalkDir;

impl Vault {
    pub fn construct_vault(root_dir: &Path) -> Result<Vault, std::io::Error> {
        let md_file_paths = WalkDir::new(root_dir)
            .into_iter()
            .filter_entry(|e| {
                !e.file_name()
                    .to_str()
                    .map(|s| s.starts_with('.') || s == "logseq") // TODO: This is a temporary fix; a hidden config is better
                    .unwrap_or(false)
            })
            .flatten()
            .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
            .collect_vec();

        let md_files: HashMap<PathBuf, MDFile> = md_file_paths
            .par_iter()
            .flat_map(|p| {
                let text = std::fs::read_to_string(p.path())?;
                let md_file = MDFile::new(&text, PathBuf::from(p.path()));

                return Ok::<(PathBuf, MDFile), std::io::Error>((p.path().into(), md_file));
            })
            .collect();

        let ropes: HashMap<PathBuf, Rope> = md_file_paths
            .par_iter()
            .flat_map(|p| {
                let text = std::fs::read_to_string(p.path())?;
                let rope = Rope::from_str(&text);

                return Ok::<(PathBuf, Rope), std::io::Error>((p.path().into(), rope));
            })
            .collect();

        Ok(Vault {
            ropes: ropes.into(),
            md_files: md_files.into(),
            root_dir: root_dir.into(),
        })
    }

    pub fn update_vault(old: &mut Vault, new_file: (&PathBuf, &str)) {
        let new_md_file = MDFile::new(new_file.1, new_file.0.clone());
        let new = old.md_files.get_mut(new_file.0);

        match new {
            Some(file) => {
                *file = new_md_file;
            }
            None => {
                old.md_files.insert(new_file.0.into(), new_md_file);
            }
        };

        let new_rope = Rope::from_str(new_file.1);
        let rope_entry = old.ropes.get_mut(new_file.0);

        match rope_entry {
            Some(rope) => {
                *rope = new_rope;
            }
            None => {
                old.ropes.insert(new_file.0.into(), new_rope);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MyHashMap<B: Hash>(HashMap<PathBuf, B>);

impl<B: Hash> Hash for MyHashMap<B> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // https://stackoverflow.com/questions/73195185/how-can-i-derive-hash-for-a-struct-containing-a-hashmap

        let mut pairs: Vec<_> = self.0.iter().collect();
        pairs.sort_by_key(|i| i.0);

        Hash::hash(&pairs, state);
    }
}

impl<B: Hash> Deref for MyHashMap<B> {
    type Target = HashMap<PathBuf, B>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// implement DerefMut
impl<B: Hash> DerefMut for MyHashMap<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<B: Hash> From<HashMap<PathBuf, B>> for MyHashMap<B> {
    fn from(value: HashMap<PathBuf, B>) -> Self {
        MyHashMap(value)
    }
}

impl Hash for Vault {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.md_files.hash(state)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// The in memory representation of the obsidian vault files. This data is exposed through an interface of methods to select the vaults data.
/// These methods do not do any interpretation or analysis of the data. That is up to the consumer of this struct. The methods are analogous to selecting on a database.
pub struct Vault {
    md_files: MyHashMap<MDFile>,
    ropes: MyHashMap<Rope>,
    root_dir: PathBuf,
}

/// Methods using vaults data
impl Vault {
    /// Select all references ([[link]] or #tag) in a file if path is some, else select all references in the vault.
    pub fn select_references<'a>(
        &'a self,
        path: Option<&'a Path>,
    ) -> Option<Vec<(&'a Path, &'a Reference)>> {
        match path {
            Some(path) => self
                .md_files
                .get(path)
                .map(|md| &md.references)
                .map(|vec| vec.iter().map(|i| (path, i)).collect()),
            None => Some(
                self.md_files
                    .iter()
                    .flat_map(|(path, md)| md.references.iter().map(|link| (path.as_path(), link)))
                    .collect(),
            ),
        }
    } // TODO: less cloning?

    pub fn select_referenceable_at_position<'a>(
        &'a self,
        path: &'a Path,
        position: Position,
    ) -> Option<Referenceable<'a>> {
        let linkable_nodes = self.select_referenceable_nodes(Some(path));
        let linkable = linkable_nodes.into_iter().find(|l| {
            let Some(range) = l.get_range() else {
                return false;
            };
            range.start.line <= position.line
                && range.end.line >= position.line
                && range.start.character <= position.character
                && range.end.character >= position.character
        })?;

        Some(linkable)
    }

    pub fn select_reference_at_position<'a>(
        &'a self,
        path: &'a Path,
        position: Position,
    ) -> Option<&Reference> {
        let links = self.select_references(Some(path))?;

        let (_path, reference) = links.into_iter().find(|&l| {
            l.1.data().range.start.line <= position.line
            && l.1.data().range.end.line >= position.line
            && l.1.data().range.start.character <= position.character // this is a bug
            && l.1.data().range.end.character >= position.character
        })?;

        Some(reference)
    }

    /// Select all linkable positions in the vault
    pub fn select_referenceable_nodes<'a>(
        &'a self,
        path: Option<&'a Path>,
    ) -> Vec<Referenceable<'a>> {
        match path {
            Some(path) => {
                let resolved_referenceables =
                    iter::once(self.md_files.get(path).map(|md| md.get_referenceables()))
                        .flatten()
                        .flatten()
                        .collect_vec();

                resolved_referenceables

                // TODO: Add unresolved referenceables
            }
            None => {
                let resolved_referenceables = self
                    .md_files
                    .values()
                    .flat_map(|file| file.get_referenceables())
                    .collect_vec();

                let resolved_referenceables_refnames: HashSet<String> = resolved_referenceables
                    .iter()
                    .filter_map(|resolved| resolved.get_refname(self.root_dir()))
                    .collect();

                let unresolved = self.select_references(None).map(|references| {
                    references
                        .iter()
                        .filter(|(_, reference)| {
                            !resolved_referenceables_refnames
                                .contains(&reference.data().reference_text)
                        })
                        .filter_map(|(_, reference)| match reference {
                            Reference::FileLink(data) => {
                                let mut path = self.root_dir().clone();
                                path.push(&reference.data().reference_text);

                                Some(Referenceable::UnresovledFile(path, &data.reference_text))
                            }
                            Reference::HeadingLink(_data, end_path, heading) => {
                                let mut path = self.root_dir().clone();
                                path.push(end_path);

                                Some(Referenceable::UnresolvedHeading(path, end_path, heading))
                            }
                            Reference::IndexedBlockLink(_data, end_path, index) => {
                                let mut path = self.root_dir().clone();
                                path.push(end_path);

                                Some(Referenceable::UnresovledIndexedBlock(
                                    path, end_path, index,
                                ))
                            }
                            Reference::Tag(..) | Reference::Footnote(..) => None,
                        })
                        .collect_vec()
                });

                resolved_referenceables
                    .into_iter()
                    .chain(unresolved.into_iter().flatten())
                    .collect()
            }
        }
    }

    pub fn select_line(&self, path: &Path, line: usize) -> Option<Vec<char>> {
        let rope = self.ropes.get(path)?;

        rope.get_line(line).map(|slice| slice.chars().collect_vec())
    }

    pub fn select_headings(&self, path: &Path) -> Option<&Vec<MDHeading>> {
        let md_file = self.md_files.get(path)?;
        let headings = &md_file.headings;
        Some(headings)
    }

    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    pub fn select_references_for_referenceable(
        &self,
        referenceable: &Referenceable,
    ) -> Option<Vec<(&Path, &Reference)>> {
        let references = self.select_references(None)?;

        Some(
            references
                .into_iter()
                .filter(|(ref_path, reference)| {
                    referenceable.matches_reference(&self.root_dir, reference, ref_path)
                })
                .collect(),
        )
    }

    pub fn select_referenceables_for_reference(
        &self,
        reference: &Reference,
        reference_path: &Path,
    ) -> Vec<Referenceable> {
        let referenceables = self.select_referenceable_nodes(None);

        referenceables
            .into_iter()
            .filter(|i| reference.references(self.root_dir(), reference_path, i))
            .collect()
    }
}

pub enum Preview {
    Text(String),

    Empty,
}

impl From<String> for Preview {
    fn from(value: String) -> Self {
        Preview::Text(value)
    }
}

use Preview::*;

impl Vault {
    pub fn select_referenceable_preview(&self, referenceable: &Referenceable) -> Option<Preview> {
        if self
            .ropes
            .get(referenceable.get_path())
            .is_some_and(|rope| rope.len_lines() == 1)
        {
            return Some(Empty);
        }

        match referenceable {
            Referenceable::Footnote(_, _) => {
                let range = referenceable.get_range()?;
                Some(
                    String::from_iter(
                        self.select_line(referenceable.get_path(), range.start.line as usize)?,
                    )
                    .into(),
                )
            }
            Referenceable::Heading(_, _) => {
                let range = referenceable.get_range()?;
                Some(
                    (range.start.line..=range.end.line + 10)
                        .filter_map(|ln| self.select_line(referenceable.get_path(), ln as usize)) // flatten those options!
                        .map(String::from_iter)
                        .join("")
                        .into(),
                )
            }
            Referenceable::IndexedBlock(_, _) => {
                let range = referenceable.get_range()?;
                self.select_line(referenceable.get_path(), range.start.line as usize)
                    .map(String::from_iter)
                    .map(Into::into)
            }
            Referenceable::File(_, _) => {
                let file_text = self.ropes.get(referenceable.get_path()).unwrap();
                Some(String::from(file_text).into())
            }
            _ => None,
        }
    }
}

fn range_to_position(rope: &Rope, range: Range<usize>) -> MyRange {
    // convert from byte offset to char offset
    let char_start = rope.byte_to_char(range.start);
    let char_end = rope.byte_to_char(range.end);

    let start_line = rope.char_to_line(char_start);
    let start_offset = char_start - rope.line_to_char(start_line);

    let end_line = rope.char_to_line(char_end);
    let end_offset = char_end - rope.line_to_char(end_line);

    tower_lsp::lsp_types::Range {
        start: Position {
            line: start_line as u32,
            character: start_offset as u32,
        },
        end: Position {
            line: end_line as u32,
            character: end_offset as u32,
        },
    }
    .into()
}

#[derive(Debug, PartialEq, Eq, Default, Hash, Clone)]
pub struct MDFile {
    references: Vec<Reference>,
    headings: Vec<MDHeading>,
    indexed_blocks: Vec<MDIndexedBlock>,
    tags: Vec<MDTag>,
    footnotes: Vec<MDFootnote>,
    path: PathBuf,
}

impl MDFile {
    fn new(text: &str, path: PathBuf) -> MDFile {
        let links = Reference::new(text);
        let headings = MDHeading::new(text);
        let indexed_blocks = MDIndexedBlock::new(text);
        let tags = MDTag::new(text);
        let footnotes = MDFootnote::new(text);

        MDFile {
            references: links,
            headings,
            indexed_blocks,
            tags,
            footnotes,
            path,
        }
    }
}

impl MDFile {
    fn get_referenceables(&self) -> Vec<Referenceable> {
        let MDFile {
            references: _,
            headings,
            indexed_blocks,
            tags,
            footnotes,
            path: _,
        } = self;

        iter::once(Referenceable::File(&self.path, self))
            .chain(
                headings
                    .iter()
                    .map(|heading| Referenceable::Heading(&self.path, heading)),
            )
            .chain(
                indexed_blocks
                    .iter()
                    .map(|block| Referenceable::IndexedBlock(&self.path, block)),
            )
            .chain(tags.iter().map(|tag| Referenceable::Tag(&self.path, tag)))
            .chain(
                footnotes
                    .iter()
                    .map(|footnote| Referenceable::Footnote(&self.path, footnote)),
            )
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Hash)]
pub struct ReferenceData {
    pub reference_text: String,
    pub display_text: Option<String>,
    pub range: MyRange,
}

type File = String;
type Specialref = String;

// TODO: I should probably make this my own hash trait so it is more clear what it does
impl Hash for Reference {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data().reference_text.hash(state)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Reference {
    Tag(ReferenceData),
    FileLink(ReferenceData),
    HeadingLink(ReferenceData, File, Specialref),
    IndexedBlockLink(ReferenceData, File, Specialref),
    Footnote(ReferenceData),
}

impl Default for Reference {
    fn default() -> Self {
        FileLink(ReferenceData::default())
    }
}

use Reference::*;

impl Reference {
    pub fn data(&self) -> &ReferenceData {
        match &self {
            Tag(data, ..) => data,
            FileLink(data, ..) => data,
            HeadingLink(data, ..) => data,
            IndexedBlockLink(data, ..) => data,
            Footnote(data) => data,
        }
    }

    pub fn matches_type(&self, other: &Reference) -> bool {
        match &other {
            Tag(..) => matches!(self, Tag(..)),
            FileLink(..) => matches!(self, FileLink(..)),
            HeadingLink(..) => matches!(self, HeadingLink(..)),
            IndexedBlockLink(..) => matches!(self, IndexedBlockLink(..)),
            Footnote(..) => matches!(self, Footnote(..)),
        }
    }

    fn new(text: &str) -> Vec<Reference> {
        static LINK_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\[\[(?<filepath>[^\[\]\|\.\#]+)(\#(?<infileref>[^\[\]\.\|]+))?(\|(?<display>[^\[\]\.\|]+))?\]\]")
                .unwrap()
        }); // A [[link]] that does not have any [ or ] in it

        let links: Vec<Reference> = LINK_RE
            .captures_iter(text)
            .flat_map(|capture| {
                match (
                    capture.get(0),
                    capture.name("filepath"),
                    capture.name("infileref"),
                    capture.name("display"),
                ) {
                    (Some(full), Some(fileref), infileref, display) => {
                        Some((full, fileref, infileref, display))
                    }
                    _ => None,
                }
            })
            .map(|linkmatch| {
                match linkmatch {
                    // Pure file reference as there is no infileref such as #... for headings or #^... for indexed blocks
                    (full, filepath, None, display) => {
                        return FileLink(ReferenceData {
                            reference_text: filepath.as_str().into(),
                            range: range_to_position(&Rope::from_str(text), full.range()),
                            display_text: display.map(|d| d.as_str().into()),
                        })
                    }
                    (full, filepath, Some(infile), display)
                        if infile.as_str().get(0..1) == Some("^") =>
                    {
                        return IndexedBlockLink(
                            ReferenceData {
                                reference_text: format!(
                                    "{}#{}",
                                    filepath.as_str(),
                                    infile.as_str()
                                ),
                                range: range_to_position(&Rope::from_str(text), full.range()),
                                display_text: display.map(|d| d.as_str().into()),
                            },
                            filepath.as_str().into(),
                            infile.as_str().into(),
                        )
                    }
                    (full, filepath, Some(infile), display) => {
                        return HeadingLink(
                            ReferenceData {
                                reference_text: format!(
                                    "{}#{}",
                                    filepath.as_str(),
                                    infile.as_str()
                                ),
                                range: range_to_position(&Rope::from_str(text), full.range()),
                                display_text: display.map(|d| d.as_str().into()),
                            },
                            filepath.as_str().into(),
                            infile.as_str().into(),
                        )
                    }
                }
            })
            .collect_vec();

        let tags: Vec<Reference> = MDTag::new(text)
            .iter()
            .map(|tag| {
                Tag(ReferenceData {
                    display_text: None,
                    range: tag.range,
                    reference_text: format!("#{}", tag.tag_ref),
                })
            })
            .collect();

        static FOOTNOTE_LINK_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"[^\[](?<full>\[(?<index>\^[^\[\] ]+)\])[^\:]").unwrap());
        let footnote_references: Vec<Reference> = FOOTNOTE_LINK_RE
            .captures_iter(text)
            .flat_map(
                |capture| match (capture.name("full"), capture.name("index")) {
                    (Some(full), Some(index)) => Some((full, index)),
                    _ => None,
                },
            )
            .map(|(outer, index)| {
                Footnote(ReferenceData {
                    reference_text: index.as_str().into(),
                    range: range_to_position(&Rope::from_str(text), outer.range()),
                    display_text: None,
                })
            })
            .collect_vec();

        links
            .into_iter()
            .chain(tags)
            .chain(footnote_references)
            .collect_vec()
    }

    pub fn references(
        &self,
        root_dir: &Path,
        file_path: &Path,
        referenceable: &Referenceable,
    ) -> bool {
        let text = &self.data().reference_text;
        match referenceable {
            &Referenceable::Tag(_, _) => {
                matches!(self, Tag(_))
                    && referenceable.get_refname(root_dir) == Some(text.to_string())
            }
            &Referenceable::Footnote(path, _footnote) => {
                matches!(self, Footnote(_))
                    && referenceable.get_refname(root_dir).as_ref() == Some(text)
                    && path.as_path() == file_path
            }
            &Referenceable::File(_path, _file) => {
                matches!(self, FileLink(_))
                    && referenceable.get_refname(root_dir).as_ref() == Some(text)
            }
            &Referenceable::Heading(_path, _file) => {
                matches!(self, HeadingLink(..))
                    && referenceable.get_refname(root_dir).as_ref() == Some(text)
            }
            &Referenceable::IndexedBlock(_path, _file) => {
                matches!(self, IndexedBlockLink(..))
                    && referenceable.get_refname(root_dir).as_ref() == Some(text)
            }
            &Referenceable::UnresovledFile(..) => {
                matches!(self, FileLink(_))
                    && referenceable.get_refname(root_dir).as_ref() == Some(text)
            }
            &Referenceable::UnresolvedHeading(..) => {
                matches!(self, HeadingLink(..))
                    && referenceable.get_refname(root_dir).as_ref() == Some(text)
            }
            &Referenceable::UnresovledIndexedBlock(..) => {
                matches!(self, IndexedBlockLink(..))
                    && referenceable.get_refname(root_dir).as_ref() == Some(text)
            }
        }
    }
}

#[derive(Eq, PartialEq, Debug, PartialOrd, Ord, Clone, Hash)]
pub struct HeadingLevel(pub usize);

impl Default for HeadingLevel {
    fn default() -> Self {
        HeadingLevel(1)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct MDHeading {
    pub heading_text: String,
    pub range: MyRange,
    pub level: HeadingLevel,
}

impl Hash for MDHeading {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.level.hash(state);
        self.heading_text.hash(state)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct MyRange(pub tower_lsp::lsp_types::Range);

impl Hash for MyRange {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.start.line.hash(state);
        self.0.start.character.hash(state);
        self.0.end.character.hash(state);
        self.0.end.character.hash(state);
    }
}

impl Deref for MyRange {
    type Target = tower_lsp::lsp_types::Range;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<tower_lsp::lsp_types::Range> for MyRange {
    fn from(range: tower_lsp::lsp_types::Range) -> Self {
        MyRange(range)
    }
}

impl MDHeading {
    fn new(text: &str) -> Vec<MDHeading> {
        static HEADING_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?<starter>#+) (?<heading_text>.+)").unwrap());

        let headings: Vec<MDHeading> = HEADING_RE
            .captures_iter(text)
            .flat_map(
                |c| match (c.get(0), c.name("heading_text"), c.name("starter")) {
                    (Some(full), Some(text), Some(starter)) => Some((full, text, starter)),
                    _ => None,
                },
            )
            .map(|(full_heading, heading_match, starter)| {
                return MDHeading {
                    heading_text: heading_match.as_str().trim_end().into(),
                    range: range_to_position(&Rope::from_str(text), full_heading.range()),
                    level: HeadingLevel(starter.as_str().len()),
                };
            })
            .collect_vec();

        headings
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MDIndexedBlock {
    index: String,
    range: MyRange,
}

impl Hash for MDIndexedBlock {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl MDIndexedBlock {
    fn new(text: &str) -> Vec<MDIndexedBlock> {
        static INDEXED_BLOCK_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r".+ (\^(?<index>\w+))").unwrap());

        let indexed_blocks: Vec<MDIndexedBlock> = INDEXED_BLOCK_RE
            .captures_iter(text)
            .flat_map(|c| match (c.get(1), c.name("index")) {
                (Some(full), Some(index)) => Some((full, index)),
                _ => None,
            })
            .map(|(full, index)| MDIndexedBlock {
                index: index.as_str().into(),
                range: range_to_position(&Rope::from_str(text), full.range()),
            })
            .collect_vec();

        indexed_blocks
    } // Make this better identify the full blocks
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct MDFootnote {
    index: String,
    footnote_text: String,
    range: MyRange,
}

impl Hash for MDFootnote {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.footnote_text.hash(state);
    }
}

impl MDFootnote {
    fn new(text: &str) -> Vec<MDFootnote> {
        // static FOOTNOTE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r".+ (\^(?<index>\w+))").unwrap());
        static FOOTNOTE_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\[(?<index>\^[^ \[\]]+)\]\:(?<text>.+)").unwrap());

        let footnotes: Vec<MDFootnote> = FOOTNOTE_RE
            .captures_iter(text)
            .flat_map(|c| match (c.get(0), c.name("index"), c.name("text")) {
                (Some(full), Some(index), Some(footnote_text)) => {
                    Some((full, index, footnote_text))
                }
                _ => None,
            })
            .map(|(full, index, footnote_text)| MDFootnote {
                footnote_text: footnote_text.as_str().trim_start().into(),
                index: index.as_str().into(),
                range: range_to_position(&Rope::from_str(text), full.range()),
            })
            .collect_vec();

        footnotes
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MDTag {
    tag_ref: String,
    range: MyRange,
}

impl Hash for MDTag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tag_ref.hash(state);
    }
}

impl MDTag {
    fn new(text: &str) -> Vec<MDTag> {
        static TAG_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(\n|\A| )(?<full>#(?<tag>[.[^ \n\#]]+))(\n|\z| )").unwrap());

        let tagged_blocks = TAG_RE
            .captures_iter(text)
            .flat_map(|c| match (c.name("full"), c.name("tag")) {
                (Some(full), Some(index)) => Some((full, index)),
                _ => None,
            })
            .filter(|(_, index)| index.as_str().chars().any(|c| c.is_alphabetic()))
            .map(|(full, index)| MDTag {
                tag_ref: index.as_str().into(),
                range: range_to_position(&Rope::from_str(text), full.range()),
            })
            .collect_vec();

        tagged_blocks
    }
}



#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/**
An Algebreic type for methods for all referenceables, which are anything able to be referenced through obsidian link or tag. These include
Files, headings, indexed blocks, tags, ...

I chose to use an enum instead of a trait as (1) I dislike the ergonomics with dynamic dyspatch, (2) It is sometimes necessary to differentiate between members of this abstraction, (3) it was convienient for this abstraction to hold the path of the referenceable for use in matching link names etc...

The vault struct is focused on presenting data from the obsidian vault through a good usable interface. The vault module as whole, however, is in change in interfacting with the obsidian syntax, which is where the methods on this enum are applicable. Obsidian has a specific linking style, and the methods on this enum provide a way to work with this syntax in a way that decouples the interpretation from other modules. The most common one method is the `is_reference` which tells if a piece of text is a refence to a particular referenceable (which is implemented differently for each type of referenceable). As a whole, this provides an abstraction around interpreting obsidian syntax; when obsidian updates syntax, code here changes and not in other places; when new referenceables are added and code is needed to interpret/match its links, code here changes and not elsewhere.
*/
pub enum Referenceable<'a> {
    File(&'a PathBuf, &'a MDFile),
    Heading(&'a PathBuf, &'a MDHeading),
    IndexedBlock(&'a PathBuf, &'a MDIndexedBlock),
    Tag(&'a PathBuf, &'a MDTag),
    Footnote(&'a PathBuf, &'a MDFootnote),
    UnresovledFile(PathBuf, &'a String),
    UnresolvedHeading(PathBuf, &'a String, &'a String),
    UnresovledIndexedBlock(PathBuf, &'a String, &'a String),
}

/// Utility function
fn get_obsidian_ref_path(root_dir: &Path, path: &Path) -> Option<String> {
    diff_paths(path, root_dir).and_then(|diff| diff.with_extension("").to_str().map(String::from))
}

impl Referenceable<'_> {
    /// Gets the generic reference name for a referenceable. This will not include any display text. If trying to determine if text is a reference of a particular referenceable, use the `is_reference` function
    pub fn get_refname(&self, root_dir: &Path) -> Option<String> {
        match self {
            Referenceable::File(path, _) => get_obsidian_ref_path(root_dir, path),
            Referenceable::Heading(path, heading) => get_obsidian_ref_path(root_dir, path)
                .map(|refpath| format!("{}#{}", refpath, heading.heading_text)),
            Referenceable::IndexedBlock(path, heading) => get_obsidian_ref_path(root_dir, path)
                .map(|refpath| format!("{}#^{}", refpath, heading.index)),
            Referenceable::Tag(_, tag) => Some(format!("#{}", tag.tag_ref)),
            Referenceable::Footnote(_, footnote) => Some(footnote.index.clone()),
            Referenceable::UnresolvedHeading(_, path, heading) => {
                Some(format!("{}#{}", path, heading))
            }
            Referenceable::UnresovledFile(_, path) => Some(path.to_string()),
            Referenceable::UnresovledIndexedBlock(_, path, index) => {
                Some(format!("{}#{}", path, index))
            }
        }
    }

    pub fn matches_reference(
        &self,
        root_dir: &Path,
        reference: &Reference,
        reference_path: &Path,
    ) -> bool {
        let text = &reference.data().reference_text;
        match &self {
            Referenceable::Tag(_, _) => {
                matches!(reference, Tag(_))
                    && self
                        .get_refname(root_dir)
                    .is_some_and(|refname| {
                        let refname_split = refname.split("/").collect_vec();
                        let text_split = text.split("/").collect_vec();

                        return text_split.get(0..refname_split.len()) == Some(&refname_split)
                    })
            }
            Referenceable::Footnote(path, _footnote) => {
                matches!(reference, Footnote(_data))
                    && self.get_refname(root_dir).as_ref() == Some(text)
                    && path.as_path() == reference_path
            }
            Referenceable::File(_path, _file) => {
                matches!(reference, FileLink(data) if Some(&data.reference_text) == self.get_refname(root_dir).as_ref())
                    || matches!(reference, HeadingLink(.., file, _) if Some(file) == self.get_refname(root_dir).as_ref())
                    || matches!(reference, IndexedBlockLink(.., file, _) if Some(file) == self.get_refname(root_dir).as_ref())
            }
            Referenceable::Heading(_path, _file) => {
                matches!(reference, HeadingLink(data, ..) if Some(&data.reference_text) == self.get_refname(root_dir).as_ref())
            }
            Referenceable::IndexedBlock(..) => {
                matches!(reference, IndexedBlockLink(.., _file, _) if Some(text) == self.get_refname(root_dir).as_ref())
            }
            Referenceable::UnresovledFile(_, _path) => {
                matches!(reference, FileLink(data) if Some(&data.reference_text) == self.get_refname(root_dir).as_ref())
                    || matches!(reference, HeadingLink(.., file, _) if Some(file) == self.get_refname(root_dir).as_ref())
                    || matches!(reference, IndexedBlockLink(.., file, _) if Some(file) == self.get_refname(root_dir).as_ref())
            }
            Referenceable::UnresolvedHeading(_, _path, _heading) => {
                matches!(reference, HeadingLink(data, ..) if Some(&data.reference_text) == self.get_refname(root_dir).as_ref())
            }
            Referenceable::UnresovledIndexedBlock(_, _path, _index) => {
                matches!(reference, IndexedBlockLink(.., _file, _) if Some(text) == self.get_refname(root_dir).as_ref())
            }
        }
    }

    pub fn get_path(&self) -> &Path {
        match self {
            &Referenceable::File(path, _) => path,
            &Referenceable::Heading(path, _) => path,
            &Referenceable::IndexedBlock(path, _) => path,
            &Referenceable::Tag(path, _) => path,
            &Referenceable::Footnote(path, _) => path,
            Referenceable::UnresovledIndexedBlock(path, ..) => path,
            Referenceable::UnresovledFile(path, ..) => path,
            Referenceable::UnresolvedHeading(path, ..) => path,
        }
    }

    pub fn get_range(&self) -> Option<MyRange> {
        match self {
            &Referenceable::File(_, _) => Some(
                tower_lsp::lsp_types::Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 1,
                    },
                }
                .into(),
            ),
            &Referenceable::Heading(_, heading) => Some(heading.range),
            &Referenceable::IndexedBlock(_, indexed_block) => Some(indexed_block.range),
            &Referenceable::Tag(_, tag) => Some(tag.range),
            &Referenceable::Footnote(_, footnote) => Some(footnote.range),
            &Referenceable::UnresovledFile(..)
            | &Referenceable::UnresolvedHeading(..)
            | &Referenceable::UnresovledIndexedBlock(..) => None,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        matches!(
            self,
            Referenceable::UnresolvedHeading(..)
                | Referenceable::UnresovledFile(..)
                | Referenceable::UnresovledIndexedBlock(..)
        )
    }
}

// tests
#[cfg(test)]
mod vault_tests {
    use std::path::{Path, PathBuf};

    use tower_lsp::lsp_types::{Position, Range};

    use crate::vault::{HeadingLevel, ReferenceData};

    use super::Reference::*;
    use super::Vault;
    use super::{MDFile, MDFootnote, MDHeading, MDIndexedBlock, MDTag, Reference, Referenceable};

    #[test]
    fn link_parsing() {
        let text = "This is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = Reference::new(text);

        let expected = vec![
            FileLink(ReferenceData {
                reference_text: "link".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 10,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 18,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
            FileLink(ReferenceData {
                reference_text: "link 2".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 19,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 29,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
            FileLink(ReferenceData {
                reference_text: "link 3".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 0,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 10,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn link_parsin_with_display_text() {
        let text = "This is a [[link|but called different]] [[link 2|222]]\n[[link 3|333]]";
        let parsed = Reference::new(text);

        let expected = vec![
            FileLink(ReferenceData {
                reference_text: "link".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 10,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 39,
                    },
                }
                .into(),
                display_text: Some("but called different".into()),
            }),
            FileLink(ReferenceData {
                reference_text: "link 2".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 40,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 54,
                    },
                }
                .into(),
                display_text: Some("222".into()),
            }),
            FileLink(ReferenceData {
                reference_text: "link 3".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 0,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 14,
                    },
                }
                .into(),
                display_text: Some("333".into()),
            }),
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn footnote_link_parsing() {
        let text = "This is a footnote[^1]

[^1]: This is not";
        let parsed = Reference::new(text);
        let expected = vec![Footnote(ReferenceData {
            reference_text: "^1".into(),
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 18,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 22,
                },
            }
            .into(),
            ..ReferenceData::default()
        })];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn link_parsing_with_png() {
        let text = "This is a png [[link.png]] [[link|display.png]]";
        let parsed = Reference::new(text);

        assert_eq!(parsed, vec![])
    }

    #[test]
    fn heading_parsing() {
        let text = r"# This is a heading

Some more text on the second line

Some text under it

some mroe text

more text


## This shoudl be a heading!";

        let parsed = MDHeading::new(text);

        let expected = vec![
            MDHeading {
                heading_text: "This is a heading".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 19,
                    },
                }
                .into(),
                ..Default::default()
            },
            MDHeading {
                heading_text: "This shoudl be a heading!".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 11,
                        character: 0,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 11,
                        character: 28,
                    },
                }
                .into(),
                level: HeadingLevel(2),
            },
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn indexed_block_parsing() {
        let text = r"# This is a heading

        Some more text on the second line fjasdkl fdkaslfjdaskl jfklas fjkldasj fkldsajfkld
        fasd fjkldasfjkldasfj kldasfj dklas
        afd asjklfdjasklfj dklasfjkdlasjfkldjasklfasd
        af djaskl
        f jdaskfjdklasfj kldsafjkldsa
        f jasdkfj dsaklfdsal ^12345

        Some text under it
        some mroe text
        more text";

        let parsed = MDIndexedBlock::new(text);

        assert_eq!(parsed[0].index, "12345")
    }

    #[test]
    fn test_linkable_reference() {
        let path = Path::new("/home/vault/test.md");
        let path_buf = path.to_path_buf();
        let md_file = MDFile::default();
        let linkable: Referenceable = Referenceable::File(&path_buf, &md_file);

        let root_dir = Path::new("/home/vault");
        let refname = linkable.get_refname(root_dir);

        assert_eq!(refname, Some("test".into()))
    }

    #[test]
    fn test_linkable_reference_heading() {
        let path = Path::new("/home/vault/test.md");
        let path_buf = path.to_path_buf();
        let md_heading = MDHeading {
            heading_text: "Test Heading".into(),
            range: tower_lsp::lsp_types::Range::default().into(),
            ..Default::default()
        };
        let linkable: Referenceable = Referenceable::Heading(&path_buf, &md_heading);

        let root_dir = Path::new("/home/vault");
        let refname = linkable.get_refname(root_dir);

        assert_eq!(refname, Some("test#Test Heading".into()))
    }

    #[test]
    fn test_linkable_reference_indexed_block() {
        let path = Path::new("/home/vault/test.md");
        let path_buf = path.to_path_buf();
        let md_indexed_block = MDIndexedBlock {
            index: "12345".into(),
            range: tower_lsp::lsp_types::Range::default().into(),
        };
        let linkable: Referenceable = Referenceable::IndexedBlock(&path_buf, &md_indexed_block);

        let root_dir = Path::new("/home/vault");
        let refname = linkable.get_refname(root_dir);

        assert_eq!(refname, Some("test#^12345".into()))
    }

    #[test]
    fn parsing_special_text() {
        let text = "’’’󰌶 is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = Reference::new(text);

        let expected = vec![
            FileLink(ReferenceData {
                reference_text: "link".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 10,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 18,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
            FileLink(ReferenceData {
                reference_text: "link 2".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 19,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 29,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
            FileLink(ReferenceData {
                reference_text: "link 3".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 0,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 1,
                        character: 10,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_construct_vault() {
        // get this projects root dir
        let mut root_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).into();
        root_dir.push("TestFiles");

        match Vault::construct_vault(&root_dir) {
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    fn test_obsidian_tag() {
        let text = r"# This is a heading

This is a #tag

and another #tag/ttagg

and a third tag#notatag [[link#not a tag]]

#MapOfContext/apworld
";
        let expected: Vec<MDTag> = vec![
            MDTag {
                tag_ref: "tag".into(),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 10,
                    },
                    end: Position {
                        line: 2,
                        character: 14,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "tag/ttagg".into(),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 12,
                    },
                    end: Position {
                        line: 4,
                        character: 22,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "MapOfContext/apworld".into(),
                range: Range {
                    start: Position {
                        line: 8,
                        character: 0,
                    },
                    end: Position {
                        line: 8,
                        character: 21,
                    },
                }
                .into(),
            },
        ];

        let parsed = MDTag::new(text);

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_obsidian_footnote() {
        let text = "[^1]: This is a footnote";
        let parsed = MDFootnote::new(text);
        let expected = vec![MDFootnote {
            index: "^1".into(),
            footnote_text: "This is a footnote".into(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 24,
                },
            }
            .into(),
        }];

        assert_eq!(parsed, expected);

        let text = r"# This is a heading

Referenced[^1]

[^1]: Footnote here

Continued

[^2]: Another footnote
[^a]:Third footnot3
";
        let parsed = MDFootnote::new(text);
        let expected = vec![
            MDFootnote {
                index: "^1".into(),
                footnote_text: "Footnote here".into(),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 4,
                        character: 19,
                    },
                }
                .into(),
            },
            MDFootnote {
                index: "^2".into(),
                footnote_text: "Another footnote".into(),
                range: Range {
                    start: Position {
                        line: 8,
                        character: 0,
                    },
                    end: Position {
                        line: 8,
                        character: 22,
                    },
                }
                .into(),
            },
            MDFootnote {
                index: "^a".into(),
                footnote_text: "Third footnot3".into(),
                range: Range {
                    start: Position {
                        line: 9,
                        character: 0,
                    },
                    end: Position {
                        line: 9,
                        character: 19,
                    },
                }
                .into(),
            },
        ];

        assert_eq!(parsed, expected)
    }
}
