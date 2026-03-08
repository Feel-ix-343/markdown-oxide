mod metadata;
mod parsing;

use std::{
    char,
    collections::{HashMap, HashSet},
    hash::Hash,
    iter,
    ops::{Deref, DerefMut, Not, Range},
    path::{Path, PathBuf},
    time::SystemTime,
};

use itertools::Itertools;
use once_cell::sync::Lazy;
use pathdiff::diff_paths;
use rayon::prelude::*;
use regex::{Captures, Match, Regex};
use ropey::Rope;
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Location, Position, SymbolInformation, SymbolKind, Url};
use walkdir::WalkDir;

impl Vault {
    pub fn construct_vault(context: &Settings, root_dir: &Path) -> Result<Vault, std::io::Error> {
        let md_file_paths = WalkDir::new(root_dir)
            .into_iter()
            .filter_entry(|e| {
                // Allow the root directory itself even if it starts with '.'
                if e.path() == root_dir {
                    return true;
                }
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
                let md_file = MDFile::new(context, &text, PathBuf::from(p.path()));

                Ok::<(PathBuf, MDFile), std::io::Error>((p.path().into(), md_file))
            })
            .collect();

        let ropes: HashMap<PathBuf, Rope> = md_file_paths
            .iter()
            .flat_map(|p| {
                let text = std::fs::read_to_string(p.path())?;
                let rope = Rope::from_str(&text);

                Ok::<(PathBuf, Rope), std::io::Error>((p.path().into(), rope))
            })
            .collect();

        Ok(Vault {
            ropes: ropes.into(),
            md_files: md_files.into(),
            root_dir: root_dir.into(),
        })
    }

    pub fn update_vault(context: &Settings, old: &mut Vault, new_file: (&PathBuf, &str)) {
        let new_md_file = MDFile::new(context, new_file.1, new_file.0.clone());
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

fn find_range(referenceable: &Referenceable) -> Option<tower_lsp::lsp_types::Range> {
    match referenceable {
        Referenceable::File(..) => Some(tower_lsp::lsp_types::Range {
            start: tower_lsp::lsp_types::Position {
                line: 0,
                character: 0,
            },
            end: tower_lsp::lsp_types::Position {
                line: 0,
                character: 1,
            },
        }),
        _ => match referenceable.get_range() {
            None => None,
            Some(a_my_range) => Some(*a_my_range),
        },
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// The in memory representation of the obsidian vault files. This data is exposed through an interface of methods to select the vaults data.
/// These methods do not do any interpretation or analysis of the data. That is up to the consumer of this struct. The methods are analogous to selecting on a database.
pub struct Vault {
    pub md_files: MyHashMap<MDFile>,
    pub ropes: MyHashMap<Rope>,
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
    }

    pub fn select_referenceable_at_position<'a>(
        &'a self,
        path: &'a Path,
        position: Position,
    ) -> Option<Referenceable<'a>> {
        // If no other referenceables are under the cursor, the file should be returned.

        let referenceable_nodes = self.select_referenceable_nodes(Some(path));

        let referenceable = referenceable_nodes
            .into_iter()
            .flat_map(|referenceable| Some((referenceable.clone(), referenceable.get_range()?)))
            .find(|(_, range)| {
                range.start.line <= position.line
                    && range.end.line >= position.line
                    && range.start.character <= position.character
                    && range.end.character >= position.character
            })
            .map(|tupl| tupl.0);

        match referenceable {
            None => self
                .md_files
                .iter()
                .find(|(iterpath, _)| *iterpath == path)
                .map(|(pathbuf, mdfile)| Referenceable::File(pathbuf, mdfile)),
            _ => referenceable,
        }
    }

    pub fn select_reference_at_position<'a>(
        &'a self,
        path: &'a Path,
        position: Position,
    ) -> Option<&'a Reference> {
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
                    .par_bridge()
                    .into_par_iter()
                    .flat_map(|file| file.get_referenceables())
                    .collect::<Vec<_>>();

                let resolved_referenceables_refnames: HashSet<String> = resolved_referenceables
                    .par_iter()
                    .flat_map(|resolved| {
                        resolved.get_refname(self.root_dir()).and_then(|refname| {
                            vec![
                                refname.to_string(),
                                format!(
                                    "{}{}",
                                    refname.link_file_key()?,
                                    refname
                                        .infile_ref
                                        .map(|refe| format!("#{}", refe))
                                        .unwrap_or("".to_string())
                                ),
                            ]
                            .into()
                        })
                    })
                    .flatten()
                    .collect();

                let unresolved = self.select_references(None).map(|references| {
                    references
                        .iter()
                        .unique_by(|(_, reference)| &reference.data().reference_text)
                        .par_bridge()
                        .into_par_iter()
                        .filter(|(_, reference)| {
                            !resolved_referenceables_refnames
                                .contains(&reference.data().reference_text)
                        })
                        .flat_map(|(_, reference)| match reference {
                            Reference::WikiFileLink(data) | Reference::MDFileLink(data) => {
                                let mut path = self.root_dir().clone();
                                path.push(&reference.data().reference_text);

                                Some(Referenceable::UnresovledFile(path, &data.reference_text))

                                // match data.reference_text.chars().collect_vec().as_slice() {

                                //     [..,'.','m','d'] =>
                                //     ['.', '/', rest @ ..]
                                //     | ['/', rest @ ..]
                                //     | rest if !rest.contains(&'.') => Some(Referenceable::UnresovledFile(path, &data.reference_text)),
                                //     _ => None
                                // }
                            }
                            Reference::WikiHeadingLink(_data, end_path, heading)
                            | Reference::MDHeadingLink(_data, end_path, heading) => {
                                let mut path = self.root_dir().clone();
                                path.push(end_path);

                                Some(Referenceable::UnresolvedHeading(path, end_path, heading))
                            }
                            Reference::WikiIndexedBlockLink(_data, end_path, index)
                            | Reference::MDIndexedBlockLink(_data, end_path, index) => {
                                let mut path = self.root_dir().clone();
                                path.push(end_path);

                                Some(Referenceable::UnresovledIndexedBlock(path, end_path, index))
                            }
                            Reference::Tag(..)
                            | Reference::Footnote(..)
                            | Reference::LinkRef(..) => None,
                        })
                        .collect::<Vec<_>>()
                });

                resolved_referenceables
                    .into_iter()
                    .chain(unresolved.into_iter().flatten())
                    .collect()
            }
        }
    }

    pub fn select_line(&self, path: &Path, line: isize) -> Option<Vec<char>> {
        let rope = self.ropes.get(path)?;

        let usize: usize = line.try_into().ok()?;

        rope.get_line(usize)
            .map(|slice| slice.chars().collect_vec())
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
                .into_par_iter()
                .filter(|(ref_path, reference)| {
                    referenceable.matches_reference(&self.root_dir, reference, ref_path)
                })
                .map(|(path, reference)| {
                    match std::fs::metadata(path).and_then(|meta| meta.modified()) {
                        Ok(modified) => (path, reference, modified),
                        Err(_) => (path, reference, SystemTime::UNIX_EPOCH),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .sorted_by_key(|(_, _, modified)| *modified)
                .rev()
                .map(|(one, two, _)| (one, two))
                .collect(),
        )
    }

    pub fn select_referenceables_for_reference(
        &self,
        reference: &Reference,
        reference_path: &Path,
    ) -> Vec<Referenceable<'_>> {
        let referenceables = self.select_referenceable_nodes(None);

        referenceables
            .into_iter()
            .filter(|i| reference.references(self.root_dir(), reference_path, i))
            .collect()
    }

    #[allow(deprecated)] // field deprecated has been deprecated in favor of using tags and will be removed in the future
    pub fn to_symbol_information(&self, referenceable: Referenceable) -> Option<SymbolInformation> {
        Some(SymbolInformation {
            name: referenceable.get_refname(self.root_dir())?.to_string(),
            kind: match referenceable {
                Referenceable::File(_, _) => SymbolKind::FILE,
                Referenceable::Tag(_, _) => SymbolKind::CONSTANT,
                _ => SymbolKind::KEY,
            },
            location: Location {
                uri: Url::from_file_path(referenceable.get_path()).ok()?,
                range: find_range(&referenceable)?,
            },
            container_name: None,
            tags: None,
            deprecated: None,
        })
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
            Referenceable::Footnote(_, _) | Referenceable::LinkRefDef(..) => {
                let range = referenceable.get_range()?;
                Some(
                    String::from_iter(
                        self.select_line(referenceable.get_path(), range.start.line as isize)?,
                    )
                    .into(),
                )
            }
            Referenceable::Heading(_, _) => {
                let range = referenceable.get_range()?;
                Some(
                    (range.start.line..=range.end.line + 10)
                        .filter_map(|ln| self.select_line(referenceable.get_path(), ln as isize)) // flatten those options!
                        .map(String::from_iter)
                        .join("")
                        .into(),
                )
            }
            Referenceable::IndexedBlock(_, _) => {
                let range = referenceable.get_range()?;
                self.select_line(referenceable.get_path(), range.start.line as isize)
                    .map(String::from_iter)
                    .map(Into::into)
            }
            Referenceable::File(_, _) => {
                Some(
                    (0..=13)
                        .filter_map(|ln| self.select_line(referenceable.get_path(), ln as isize)) // flatten those options!
                        .map(String::from_iter)
                        .join("")
                        .into(),
                )
            }
            Referenceable::Tag(_, _) => None,
            Referenceable::UnresovledFile(_, _) => None,
            Referenceable::UnresolvedHeading(_, _, _) => None,
            Referenceable::UnresovledIndexedBlock(_, _, _) => None,
        }
    }

    pub fn select_blocks(&self) -> Vec<Block<'_>> {
        self.ropes
            .par_iter()
            .map(|(path, rope)| {
                rope.lines()
                    .enumerate()
                    .flat_map(|(i, line)| {
                        let string = line.as_str()?;

                        Some(Block {
                            text: string.trim(),
                            range: MyRange(tower_lsp::lsp_types::Range {
                                start: Position {
                                    line: i as u32,
                                    character: 0,
                                },
                                end: Position {
                                    line: i as u32,
                                    character: string.len() as u32,
                                },
                            }),
                            file: path,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .filter(|block| !block.text.is_empty())
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Copy)]
pub struct Block<'a> {
    pub text: &'a str,
    pub range: MyRange,
    pub file: &'a Path,
}

impl AsRef<str> for Block<'_> {
    fn as_ref(&self) -> &str {
        self.text
    }
}

pub trait Rangeable {
    fn range(&self) -> &MyRange;
    fn includes(&self, other: &impl Rangeable) -> bool {
        let self_range = self.range();
        let other_range = other.range();

        (self_range.start.line < other_range.start.line
            || (self_range.start.line == other_range.start.line
                && self_range.start.character <= other_range.start.character))
            && (self_range.end.line > other_range.end.line
                || (self_range.end.line == other_range.end.line
                    && self_range.end.character >= other_range.end.character))
    }

    fn includes_position(&self, position: Position) -> bool {
        let range = self.range();
        (range.start.line < position.line
            || (range.start.line == position.line && range.start.character <= position.character))
            && (range.end.line > position.line
                || (range.end.line == position.line && range.end.character >= position.character))
    }
}

impl Rangeable for MDHeading {
    fn range(&self) -> &MyRange {
        &self.range
    }
}

impl Rangeable for MDFootnote {
    fn range(&self) -> &MyRange {
        &self.range
    }
}

impl Rangeable for MDIndexedBlock {
    fn range(&self) -> &MyRange {
        &self.range
    }
}

impl Rangeable for MDTag {
    fn range(&self) -> &MyRange {
        &self.range
    }
}

impl Rangeable for MDLinkReferenceDefinition {
    fn range(&self) -> &MyRange {
        &self.range
    }
}

impl Rangeable for Reference {
    fn range(&self) -> &MyRange {
        &self.range
    }
}

#[derive(Debug, PartialEq, Eq, Default, Hash, Clone)]
pub struct MDFile {
    pub references: Vec<Reference>,
    pub headings: Vec<MDHeading>,
    pub indexed_blocks: Vec<MDIndexedBlock>,
    pub tags: Vec<MDTag>,
    pub footnotes: Vec<MDFootnote>,
    pub path: PathBuf,
    pub link_reference_definitions: Vec<MDLinkReferenceDefinition>,
    pub metadata: Option<MDMetadata>,
    pub codeblocks: Vec<MDCodeBlock>,
}

impl MDFile {
    fn new(context: &Settings, text: &str, path: PathBuf) -> MDFile {
        let code_blocks = MDCodeBlock::new(text).collect_vec();
        let file_name = path
            .file_stem()
            .expect("file should have file stem")
            .to_str()
            .unwrap_or_default();
        let links = match context {
            Settings {
                references_in_codeblocks: false,
                ..
            } => Reference::new(text, file_name)
                .filter(|it| !code_blocks.iter().any(|codeblock| codeblock.includes(it)))
                .collect_vec(),
            _ => Reference::new(text, file_name).collect_vec(),
        };
        let headings = MDHeading::new(text)
            .filter(|it| !code_blocks.iter().any(|codeblock| codeblock.includes(it)));
        let footnotes = MDFootnote::new(text)
            .filter(|it| !code_blocks.iter().any(|codeblock| codeblock.includes(it)));
        let link_refs = MDLinkReferenceDefinition::new(text)
            .filter(|it| !code_blocks.iter().any(|codeblock| codeblock.includes(it)));
        let indexed_blocks = MDIndexedBlock::new(text)
            .filter(|it| !code_blocks.iter().any(|codeblock| codeblock.includes(it)));
        let tags = match context {
            Settings {
                tags_in_codeblocks: false,
                ..
            } => MDTag::new(text)
                .filter(|it| !code_blocks.iter().any(|codeblock| codeblock.includes(it)))
                .collect_vec(),
            _ => MDTag::new(text).collect_vec(),
        };
        let metadata = MDMetadata::new(text);

        MDFile {
            references: links,
            headings: headings.collect(),
            indexed_blocks: indexed_blocks.collect(),
            tags,
            footnotes: footnotes.collect(),
            path,
            link_reference_definitions: link_refs.collect(),
            metadata,
            codeblocks: code_blocks,
        }
    }

    pub fn file_name(&self) -> Option<&str> {
        self.path.file_stem()?.to_str()
    }
}

impl MDFile {
    fn get_referenceables(&self) -> Vec<Referenceable<'_>> {
        let MDFile {
            references: _,
            headings,
            indexed_blocks,
            tags,
            footnotes,
            path: _,
            link_reference_definitions,
            metadata: _,
            codeblocks: _,
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
            .chain(
                link_reference_definitions
                    .iter()
                    .map(|link_ref| Referenceable::LinkRefDef(&self.path, link_ref)),
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
    WikiFileLink(ReferenceData),
    WikiHeadingLink(ReferenceData, File, Specialref),
    WikiIndexedBlockLink(ReferenceData, File, Specialref),
    MDFileLink(ReferenceData),
    MDHeadingLink(ReferenceData, File, Specialref),
    MDIndexedBlockLink(ReferenceData, File, Specialref),
    Footnote(ReferenceData),
    LinkRef(ReferenceData),
}

impl Deref for Reference {
    type Target = ReferenceData;
    fn deref(&self) -> &Self::Target {
        self.data()
    }
}

impl Default for Reference {
    fn default() -> Self {
        WikiFileLink(ReferenceData::default())
    }
}

use Reference::*;

use crate::config::Settings;

use self::{metadata::MDMetadata, parsing::MDCodeBlock};

impl Reference {
    pub fn data(&self) -> &ReferenceData {
        match &self {
            Tag(data, ..) => data,
            WikiFileLink(data, ..) => data,
            WikiHeadingLink(data, ..) => data,
            WikiIndexedBlockLink(data, ..) => data,
            Footnote(data) => data,
            MDFileLink(data, ..) => data,
            MDHeadingLink(data, ..) => data,
            MDIndexedBlockLink(data, ..) => data,
            LinkRef(data, ..) => data,
        }
    }

    pub fn matches_type(&self, other: &Reference) -> bool {
        match &other {
            Tag(..) => matches!(self, Tag(..)),
            WikiFileLink(..) => matches!(self, WikiFileLink(..)),
            WikiHeadingLink(..) => matches!(self, WikiHeadingLink(..)),
            WikiIndexedBlockLink(..) => matches!(self, WikiIndexedBlockLink(..)),
            Footnote(..) => matches!(self, Footnote(..)),
            MDFileLink(..) => matches!(self, MDFileLink(..)),
            MDHeadingLink(..) => matches!(self, MDHeadingLink(..)),
            MDIndexedBlockLink(..) => matches!(self, MDIndexedBlockLink(..)),
            LinkRef(..) => matches!(self, LinkRef(..)),
        }
    }

    pub fn new<'a>(text: &'a str, file_name: &'a str) -> impl Iterator<Item = Reference> + 'a {
        static WIKI_LINK_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\[\[(?<filepath>[^\[\]\|\.\#]+)?(\#(?<infileref>[^\[\]\.\|]+))?(?<ending>\.[^\# <>]+)?(\|(?<display>[^\[\]\.\|]+))?\]\]")

                .unwrap()
        }); // A [[link]] that does not have any [ or ] in it

        let wiki_links = WIKI_LINK_RE
            .captures_iter(text)
            .filter(|captures| {
                matches!(
                    captures.name("ending").map(|ending| ending.as_str()),
                    Some(".md") | None
                )
            })
            .flat_map(RegexTuple::new)
            .flat_map(|regextuple| {
                generic_link_constructor::<WikiReferenceConstructor>(text, file_name, regextuple)
            });

        static MD_LINK_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\[(?<display>[^\[\]\.]*?)\]\(<?(?<filepath>(\.?\/)?[^\[\]\|\.\#<>]+?)?(?<ending>\.[^\# <>]+?)?(\#(?<infileref>[^\[\]\.\|<>]+?))?>?\)")
                .expect("MD Link Not Constructing")
        }); // [display](relativePath)

        let md_links = MD_LINK_RE
            .captures_iter(text)
            .filter(|captures| {
                matches!(
                    captures.name("ending").map(|ending| ending.as_str()),
                    Some(".md") | None
                )
            })
            .flat_map(RegexTuple::new)
            .flat_map(|regextuple| {
                generic_link_constructor::<MDReferenceConstructor>(text, file_name, regextuple)
            });

        let tags = MDTag::new(text).map(|tag| {
            Tag(ReferenceData {
                display_text: None,
                range: tag.range,
                reference_text: format!("#{}", tag.tag_ref),
            })
        });

        static FOOTNOTE_LINK_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?<start>\[?)(?<full>\[(?<index>\^[^\[\] ]+)\])(?<end>:?)").unwrap()
        });
        let footnote_references = FOOTNOTE_LINK_RE
            .captures_iter(text)
            .filter(|capture| matches!(
                (capture.name("start"), capture.name("end")),
                (Some(start), Some(end)) if !start.as_str().starts_with('[') && !end.as_str().ends_with(':'))
            )
            .flat_map(
                |capture| match (capture.name("full"), capture.name("index")) {
                    (Some(full), Some(index)) => Some((full, index)),
                    _ => None,
                },
            )
            .map(|(outer, index)| {
                Footnote(ReferenceData {
                    reference_text: index.as_str().into(),
                    range: MyRange::from_range(&Rope::from_str(text), outer.range()),
                    display_text: None,
                })
            });

        let link_ref_references: Vec<Reference> = if MDLinkReferenceDefinition::new(text)
            .collect_vec()
            .is_empty()
            .not()
        {
            static LINK_REF_RE: Lazy<Regex> = Lazy::new(|| {
                Regex::new(r"([^\[]|^)(?<full>\[(?<index>[^\^][^\[\] ]+)\])([^\]\(\:]|$)").unwrap()
            });

            let link_ref_references: Vec<Reference> = LINK_REF_RE
                .captures_iter(text)
                .par_bridge()
                .flat_map(
                    |capture| match (capture.name("full"), capture.name("index")) {
                        (Some(full), Some(index)) => Some((full, index)),
                        _ => None,
                    },
                )
                .map(|(outer, index)| {
                    LinkRef(ReferenceData {
                        reference_text: index.as_str().into(),
                        range: MyRange::from_range(&Rope::from_str(text), outer.range()),
                        display_text: None,
                    })
                })
                .collect::<Vec<_>>();

            link_ref_references
        } else {
            vec![]
        };

        wiki_links
            .into_iter()
            .chain(md_links)
            .chain(tags)
            .chain(footnote_references)
            .chain(link_ref_references)
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
                match self {
                    Tag(..) => {
                        referenceable
                            .get_refname(root_dir)
                            .map(|thing| thing.to_string())
                            == Some(text.to_string())
                    }

                    WikiFileLink(_) => false,
                    WikiHeadingLink(_, _, _) => false,
                    WikiIndexedBlockLink(_, _, _) => false,
                    MDFileLink(_) => false,
                    MDHeadingLink(_, _, _) => false,
                    MDIndexedBlockLink(_, _, _) => false,
                    Footnote(_) => false,
                    LinkRef(_) => false, // (no I don't write all of these by hand; I use rust-analyzers code action; I do this because when I add new item to the Reference enum, I want workspace errors everywhere relevant)
                }
            }
            &Referenceable::Footnote(path, _footnote) => match self {
                Footnote(..) => {
                    referenceable.get_refname(root_dir).as_deref() == Some(text)
                        && path.as_path() == file_path
                }
                Tag(_) => false,
                WikiFileLink(_) => false,
                WikiHeadingLink(_, _, _) => false,
                WikiIndexedBlockLink(_, _, _) => false,
                MDFileLink(_) => false,
                MDHeadingLink(_, _, _) => false,
                MDIndexedBlockLink(_, _, _) => false,
                LinkRef(_) => false,
            },
            &Referenceable::File(..) | &Referenceable::UnresovledFile(..) => match self {
                MDFileLink(ReferenceData {
                    reference_text: file_ref_text,
                    ..
                })
                | WikiFileLink(ReferenceData {
                    reference_text: file_ref_text,
                    ..
                }) => matches_path_or_file(file_ref_text, referenceable.get_refname(root_dir)),
                Tag(_) => false,
                WikiHeadingLink(_, _, _) => false,
                WikiIndexedBlockLink(_, _, _) => false,
                MDHeadingLink(_, _, _) => false,
                MDIndexedBlockLink(_, _, _) => false,
                Footnote(_) => false,
                LinkRef(_) => false,
            },
            &Referenceable::Heading(
                ..,
                MDHeading {
                    heading_text: infile_ref,
                    ..
                },
            )
            | &Referenceable::UnresolvedHeading(.., infile_ref)
            | &Referenceable::IndexedBlock(
                ..,
                MDIndexedBlock {
                    index: infile_ref, ..
                },
            )
            | &Referenceable::UnresovledIndexedBlock(.., infile_ref) => match self {
                WikiHeadingLink(.., file_ref_text, link_infile_ref)
                | WikiIndexedBlockLink(.., file_ref_text, link_infile_ref)
                | MDHeadingLink(.., file_ref_text, link_infile_ref)
                | MDIndexedBlockLink(.., file_ref_text, link_infile_ref) => {
                    matches_path_or_file(file_ref_text, referenceable.get_refname(root_dir))
                        && link_infile_ref.to_lowercase() == infile_ref.to_lowercase()
                }
                Tag(_) => false,
                WikiFileLink(_) => false,
                MDFileLink(_) => false,
                Footnote(_) => false,
                LinkRef(_) => false,
            },
            Referenceable::LinkRefDef(path, _link_ref) => match self {
                Tag(_) => false,
                WikiFileLink(_) => false,
                WikiHeadingLink(_, _, _) => false,
                WikiIndexedBlockLink(_, _, _) => false,
                MDFileLink(_) => false,
                MDHeadingLink(_, _, _) => false,
                MDIndexedBlockLink(_, _, _) => false,
                Footnote(_) => false,
                LinkRef(data) => {
                    Some(data.reference_text.to_lowercase())
                        == referenceable
                            .get_refname(root_dir)
                            .as_deref()
                            .map(|string| string.to_lowercase())
                        && file_path == *path
                }
            },
        }
    }
}

struct RegexTuple<'a> {
    range: Match<'a>,
    file_path: Option<Match<'a>>,
    infile_ref: Option<Match<'a>>,
    display_text: Option<Match<'a>>,
}

impl RegexTuple<'_> {
    fn new(capture: Captures) -> Option<RegexTuple> {
        match (
            capture.get(0),
            capture.name("filepath"),
            capture.name("infileref"),
            capture.name("display"),
        ) {
            (Some(range), file_path, infile_ref, display_text) => Some(RegexTuple {
                range,
                file_path,
                infile_ref,
                display_text,
            }),
            _ => None,
        }
    }
}

trait ParseableReferenceConstructor {
    fn new_heading(data: ReferenceData, path: &str, heading: &str) -> Reference;
    fn new_file_link(data: ReferenceData) -> Reference;
    fn new_indexed_block_link(data: ReferenceData, path: &str, index: &str) -> Reference;
} // TODO: Turn this into a macro

struct WikiReferenceConstructor;
struct MDReferenceConstructor;

impl ParseableReferenceConstructor for WikiReferenceConstructor {
    fn new_heading(data: ReferenceData, path: &str, heading: &str) -> Reference {
        Reference::WikiHeadingLink(data, path.into(), heading.into())
    }
    fn new_file_link(data: ReferenceData) -> Reference {
        Reference::WikiFileLink(data)
    }
    fn new_indexed_block_link(data: ReferenceData, path: &str, index: &str) -> Reference {
        Reference::WikiIndexedBlockLink(data, path.into(), index.into())
    }
}

impl ParseableReferenceConstructor for MDReferenceConstructor {
    fn new_heading(data: ReferenceData, path: &str, heading: &str) -> Reference {
        Reference::MDHeadingLink(data, path.into(), heading.into())
    }
    fn new_file_link(data: ReferenceData) -> Reference {
        Reference::MDFileLink(data)
    }
    fn new_indexed_block_link(data: ReferenceData, path: &str, index: &str) -> Reference {
        Reference::MDIndexedBlockLink(data, path.into(), index.into())
    }
}

fn generic_link_constructor<T: ParseableReferenceConstructor>(
    text: &str,
    file_name: &str,
    RegexTuple {
        range,
        file_path,
        infile_ref,
        display_text,
    }: RegexTuple,
) -> Option<Reference> {
    if file_path.is_some_and(|path| {
        path.as_str().starts_with("http://")
            || path.as_str().starts_with("https://")
            || path.as_str().starts_with("data:")
    }) {
        return None;
    }

    let decoded_filepath = file_path
        .map(|file_path| {
            let file_path = file_path.as_str();
            urlencoding::decode(file_path).map_or_else(|_| file_path.to_string(), |d| d.to_string())
        })
        .unwrap_or_else(|| file_name.to_string());

    match (range, decoded_filepath.as_str(), infile_ref, display_text) {
        // Pure file reference as there is no infileref such as #... for headings or #^... for indexed blocks
        (full, filepath, None, display) => Some(T::new_file_link(ReferenceData {
            reference_text: filepath.into(),
            range: MyRange::from_range(&Rope::from_str(text), full.range()),
            display_text: display.map(|d| d.as_str().into()),
        })),
        (full, filepath, Some(infile), display) if infile.as_str().get(0..1) == Some("^") => {
            Some(T::new_indexed_block_link(
                ReferenceData {
                    reference_text: format!("{}#{}", filepath, infile.as_str()),
                    range: MyRange::from_range(&Rope::from_str(text), full.range()),
                    display_text: display.map(|d| d.as_str().into()),
                },
                filepath,
                &infile.as_str()[1..], // drop the ^ for the index
            ))
        }
        (full, filepath, Some(infile), display) => Some(T::new_heading(
            ReferenceData {
                reference_text: format!("{}#{}", filepath, infile.as_str()),
                range: MyRange::from_range(&Rope::from_str(text), full.range()),
                display_text: display.map(|d| d.as_str().into()),
            },
            filepath,
            infile.as_str(),
        )),
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

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct MyRange(pub tower_lsp::lsp_types::Range);

impl MyRange {
    pub fn from_range(rope: &Rope, range: Range<usize>) -> MyRange {
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
}

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
    fn new(text: &str) -> impl Iterator<Item = MDHeading> + '_ {
        static HEADING_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?<starter>#+) (?<heading_text>.+)").unwrap());

        let headings = HEADING_RE
            .captures_iter(text)
            .flat_map(
                |c| match (c.get(0), c.name("heading_text"), c.name("starter")) {
                    (Some(full), Some(text), Some(starter)) => Some((full, text, starter)),
                    _ => None,
                },
            )
            .map(|(full_heading, heading_match, starter)| MDHeading {
                heading_text: heading_match.as_str().trim_end().into(),
                range: MyRange::from_range(&Rope::from_str(text), full_heading.range()),
                level: HeadingLevel(starter.as_str().len()),
            });

        headings
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MDIndexedBlock {
    /// THe index of the block; does not include '^'
    pub index: String,
    pub range: MyRange,
}

impl Hash for MDIndexedBlock {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl MDIndexedBlock {
    fn new(text: &str) -> impl Iterator<Item = MDIndexedBlock> + '_ {
        static INDEXED_BLOCK_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r".+ (\^(?<index>\w+))").unwrap());

        let indexed_blocks = INDEXED_BLOCK_RE
            .captures_iter(text)
            .flat_map(|c| match (c.get(1), c.name("index")) {
                (Some(full), Some(index)) => Some((full, index)),
                _ => None,
            })
            .map(|(full, index)| MDIndexedBlock {
                index: index.as_str().into(),
                range: MyRange::from_range(&Rope::from_str(text), full.range()),
            });

        indexed_blocks
    } // Make this better identify the full blocks
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct MDFootnote {
    pub index: String,
    pub footnote_text: String,
    pub range: MyRange,
}

impl Hash for MDFootnote {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.footnote_text.hash(state);
    }
}

impl MDFootnote {
    fn new(text: &str) -> impl Iterator<Item = MDFootnote> + '_ {
        // static FOOTNOTE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r".+ (\^(?<index>\w+))").unwrap());
        static FOOTNOTE_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\[(?<index>\^[^ \[\]]+)\]\:(?<text>.+)").unwrap());

        let footnotes = FOOTNOTE_RE
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
                range: MyRange::from_range(&Rope::from_str(text), full.range()),
            });

        footnotes
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MDTag {
    pub tag_ref: String,
    pub range: MyRange,
}

impl Hash for MDTag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tag_ref.hash(state);
    }
}

impl MDTag {
    fn new(text: &str) -> impl Iterator<Item = MDTag> + '_ {
        static TAG_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"(?x)
                # 1. Boundary assertion: The tag must be preceded by the start of the string, a newline, or whitespace.
                #    Using a non-capturing group (?:...) for efficiency.
                (?: \A | \n | \s )
        
                # 2. <full> capturing group: Captures the entire tag, including the '#' character.
                (?<full>
                    \#          # Matches the literal '#' character.
                    
                    # 3. <tag> capturing group: Captures the actual content of the tag.
                    (?<tag>
                        # First character of the tag:
                        # Cannot be a digit. Can be letters (Unicode), underscore, hyphen, slash, or various quotes.
                        [\p{L}_/'"‘’“”-]
        
                        # Subsequent characters of the tag:
                        # Can be letters (Unicode), digits, underscore, hyphen, slash, or various quotes.
                        [\p{L}0-9_/'"‘’“”-]*
                    )
                )
    "#).unwrap()
        });

        let tagged_blocks = TAG_RE
            .captures_iter(text)
            .flat_map(|c| match (c.name("full"), c.name("tag")) {
                (Some(full), Some(index)) => Some((full, index)),
                _ => None,
            })
            .filter(|(_, index)| index.as_str().chars().any(|c| c.is_alphabetic()))
            .map(|(full, index)| MDTag {
                tag_ref: index.as_str().into(),
                range: MyRange::from_range(&Rope::from_str(text), full.range()),
            });

        tagged_blocks
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct MDLinkReferenceDefinition {
    pub link_ref_name: String,
    pub range: MyRange,
    pub url: String,
    pub title: Option<String>,
}

impl MDLinkReferenceDefinition {
    fn new(text: &str) -> impl Iterator<Item = MDLinkReferenceDefinition> + '_ {
        static REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\[(?<index>[^\^][^ \[\]]+)\]\:(?<text>.+)").unwrap());

        let result = REGEX
            .captures_iter(text)
            .flat_map(|c| match (c.get(0), c.name("index"), c.name("text")) {
                (Some(full), Some(index), Some(text)) => Some((full, index, text)),
                _ => None,
            })
            .flat_map(|(full, index, url)| {
                Some(MDLinkReferenceDefinition {
                    link_ref_name: index.as_str().to_string(),
                    range: MyRange::from_range(&Rope::from_str(text), full.range()),
                    url: url.as_str().trim().to_string(),
                    title: None,
                })
            });

        result
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
    // TODO: Get rid of useless path here
    UnresovledFile(PathBuf, &'a String),
    UnresolvedHeading(PathBuf, &'a String, &'a String),
    /// full path, link path, index (without ^)
    UnresovledIndexedBlock(PathBuf, &'a String, &'a String),
    LinkRefDef(&'a PathBuf, &'a MDLinkReferenceDefinition),
}

/// Utility function
pub fn get_obsidian_ref_path(root_dir: &Path, path: &Path) -> Option<String> {
    diff_paths(path, root_dir).and_then(|diff| diff.with_extension("").to_str().map(String::from))
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Refname {
    pub full_refname: String,
    pub path: Option<String>,
    pub infile_ref: Option<String>,
}

impl Refname {
    pub fn link_file_key(&self) -> Option<String> {
        let path = &self.path.clone()?;

        let last = path.split('/').next_back()?;

        Some(last.to_string())
    }
}

impl Deref for Refname {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.full_refname
    }
}

impl From<String> for Refname {
    fn from(value: String) -> Self {
        Refname {
            full_refname: value.clone(),
            ..Default::default()
        }
    }
}

impl From<&str> for Refname {
    fn from(value: &str) -> Self {
        Refname {
            full_refname: value.to_string(),
            ..Default::default()
        }
    }
}

impl Referenceable<'_> {
    /// Gets the generic reference name for a referenceable. This will not include any display text. If trying to determine if text is a reference of a particular referenceable, use the `is_reference` function
    pub fn get_refname(&self, root_dir: &Path) -> Option<Refname> {
        match self {
            Referenceable::File(path, _) => {
                get_obsidian_ref_path(root_dir, path).map(|string| Refname {
                    full_refname: string.to_owned(),
                    path: string.to_owned().into(),
                    ..Default::default()
                })
            }

            Referenceable::Heading(path, heading) => get_obsidian_ref_path(root_dir, path)
                .map(|refpath| {
                    (
                        refpath.clone(),
                        format!("{}#{}", refpath, heading.heading_text),
                    )
                })
                .map(|(path, full_refname)| Refname {
                    full_refname,
                    path: path.into(),
                    infile_ref: <std::string::String as Clone>::clone(&heading.heading_text).into(),
                }),

            Referenceable::IndexedBlock(path, index) => get_obsidian_ref_path(root_dir, path)
                .map(|refpath| (refpath.clone(), format!("{}#^{}", refpath, index.index)))
                .map(|(path, full_refname)| Refname {
                    full_refname,
                    path: path.into(),
                    infile_ref: format!("^{}", index.index).into(),
                }),

            Referenceable::Tag(_, tag) => Some(Refname {
                full_refname: format!("#{}", tag.tag_ref),
                path: Some(tag.tag_ref.clone()),
                infile_ref: None,
            }),

            Referenceable::Footnote(_, footnote) => Some(footnote.index.clone().into()),

            Referenceable::UnresolvedHeading(_, path, heading) => {
                Some(format!("{}#{}", path, heading)).map(|full_ref| Refname {
                    full_refname: full_ref,
                    path: path.to_string().into(),
                    infile_ref: heading.to_string().into(),
                })
            }

            Referenceable::UnresovledFile(_, path) => Some(Refname {
                full_refname: path.to_string(),
                path: Some(path.to_string()),
                ..Default::default()
            }),

            Referenceable::UnresovledIndexedBlock(_, path, index) => {
                Some(format!("{}#^{}", path, index)).map(|full_ref| Refname {
                    full_refname: full_ref,
                    path: path.to_string().into(),
                    infile_ref: format!("^{}", index).into(),
                })
            }
            Referenceable::LinkRefDef(_, refdef) => Some(Refname {
                full_refname: refdef.link_ref_name.clone(),
                infile_ref: None,
                path: None,
            }),
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
                    && self.get_refname(root_dir).is_some_and(|refname| {
                        let refname_split = refname.split('/').collect_vec();
                        let text_split = text.split('/').collect_vec();

                        text_split.get(0..refname_split.len()) == Some(&refname_split)
                    })
            }
            Referenceable::Footnote(path, _footnote) => match reference {
                Footnote(..) => {
                    self.get_refname(root_dir).as_deref() == Some(text)
                        && path.as_path() == reference_path
                }
                MDFileLink(..) => false,
                Tag(_) => false,
                WikiFileLink(_) => false,
                WikiHeadingLink(_, _, _) => false,
                WikiIndexedBlockLink(_, _, _) => false,
                MDHeadingLink(_, _, _) => false,
                MDIndexedBlockLink(_, _, _) => false,
                LinkRef(_) => false,
            },
            Referenceable::File(..) | Referenceable::UnresovledFile(..) => match reference {
                WikiFileLink(ReferenceData {
                    reference_text: file_ref_text,
                    ..
                })
                | WikiHeadingLink(.., file_ref_text, _)
                | WikiIndexedBlockLink(.., file_ref_text, _)
                | MDFileLink(ReferenceData {
                    reference_text: file_ref_text,
                    ..
                })
                | MDHeadingLink(.., file_ref_text, _)
                | MDIndexedBlockLink(.., file_ref_text, _) => {
                    matches_path_or_file(file_ref_text, self.get_refname(root_dir))
                }
                Tag(_) => false,
                Footnote(_) => false,
                LinkRef(_) => false,
            },

            _ => reference.references(root_dir, reference_path, self),
        }
    }

    pub fn get_path(&self) -> &Path {
        match self {
            Referenceable::File(path, _) => path,
            Referenceable::Heading(path, _) => path,
            Referenceable::IndexedBlock(path, _) => path,
            Referenceable::Tag(path, _) => path,
            Referenceable::Footnote(path, _) => path,
            Referenceable::UnresovledIndexedBlock(path, ..) => path,
            Referenceable::UnresovledFile(path, ..) => path,
            Referenceable::UnresolvedHeading(path, ..) => path,
            Referenceable::LinkRefDef(path, ..) => path,
        }
    }

    pub fn get_range(&self) -> Option<MyRange> {
        match self {
            Referenceable::File(_, _) => None,
            Referenceable::Heading(_, heading) => Some(heading.range),
            Referenceable::IndexedBlock(_, indexed_block) => Some(indexed_block.range),
            Referenceable::Tag(_, tag) => Some(tag.range),
            Referenceable::Footnote(_, footnote) => Some(footnote.range),
            Referenceable::LinkRefDef(_, refdef) => Some(refdef.range),
            Referenceable::UnresovledFile(..)
            | Referenceable::UnresolvedHeading(..)
            | Referenceable::UnresovledIndexedBlock(..) => None,
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

fn matches_path_or_file(file_ref_text: &str, refname: Option<Refname>) -> bool {
    (|| {
        let refname = refname?;
        let refname_path = refname.path.clone()?; // this function should not be used for tags, ... only for heading, files, indexed blocks

        if file_ref_text.contains('/') {
            let file_ref_text = file_ref_text.replace(r"%20", " ");
            let file_ref_text = file_ref_text.replace(r"\ ", " ");

            let chars: Vec<char> = file_ref_text.chars().collect();
            match chars.as_slice() {
                &['.', '/', ref path @ ..] | &['/', ref path @ ..] => {
                    Some(String::from_iter(path) == refname_path)
                }
                path => Some(String::from_iter(path) == refname_path),
            }
        } else {
            let last_segment = refname.link_file_key()?;

            Some(file_ref_text.to_lowercase() == last_segment.to_lowercase())
        }
    })()
    .is_some_and(|b| b)
}

// tests
#[cfg(test)]
mod vault_tests {
    use std::path::Path;

    use itertools::Itertools;
    use tower_lsp::lsp_types::{Position, Range};

    use crate::vault::{HeadingLevel, MyRange, ReferenceData};
    use crate::vault::{MDLinkReferenceDefinition, Refname};

    use super::Reference::*;
    use super::{MDFile, MDFootnote, MDHeading, MDIndexedBlock, MDTag, Reference, Referenceable};

    #[test]
    fn wiki_link_parsing() {
        let text = "This is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = Reference::new(text, "test").collect_vec();

        let expected = vec![
            WikiFileLink(ReferenceData {
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
            WikiFileLink(ReferenceData {
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
            WikiFileLink(ReferenceData {
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
    fn wiki_link_heading_parsing() {
        let text = "This is a [[link#heading]]";
        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![WikiHeadingLink(
            ReferenceData {
                reference_text: "link#heading".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 10,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 26,
                    },
                }
                .into(),
                ..ReferenceData::default()
            },
            "link".into(),
            "heading".into(),
        )];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn wiki_link_indexedblock_parsing() {
        let text = "This is a [[link#^index1]]";
        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![WikiIndexedBlockLink(
            ReferenceData {
                reference_text: "link#^index1".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 10,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 26,
                    },
                }
                .into(),
                ..ReferenceData::default()
            },
            "link".into(),
            "index1".into(),
        )];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn wiki_link_parsin_with_display_text() {
        let text = "This is a [[link|but called different]] [[link 2|222]]\n[[link 3|333]]";
        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![
            WikiFileLink(ReferenceData {
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
            WikiFileLink(ReferenceData {
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
            WikiFileLink(ReferenceData {
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
    fn md_link_parsing() {
        let text = "Test text test text [link](path/to/link)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDFileLink(ReferenceData {
            reference_text: "path/to/link".into(),
            display_text: Some("link".into()),
            range: Range {
                start: Position {
                    line: 0,
                    character: 20,
                },
                end: Position {
                    line: 0,
                    character: 40,
                },
            }
            .into(),
        })];

        assert_eq!(parsed, expected);

        let text = "Test text test text [link](./path/to/link)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDFileLink(ReferenceData {
            reference_text: "./path/to/link".into(),
            display_text: Some("link".into()),
            range: Range {
                start: Position {
                    line: 0,
                    character: 20,
                },
                end: Position {
                    line: 0,
                    character: 42,
                },
            }
            .into(),
        })];

        assert_eq!(parsed, expected);

        let text = "Test text test text [link](./path/to/link.md)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDFileLink(ReferenceData {
            reference_text: "./path/to/link".into(),
            display_text: Some("link".into()),
            range: Range {
                start: Position {
                    line: 0,
                    character: 20,
                },
                end: Position {
                    line: 0,
                    character: 45,
                },
            }
            .into(),
        })];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn advanced_md_link_parsing() {
        let text = "Test text test text [link](<path to/link>)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDFileLink(ReferenceData {
            reference_text: "path to/link".into(),
            display_text: Some("link".into()),
            range: Range {
                start: Position {
                    line: 0,
                    character: 20,
                },
                end: Position {
                    line: 0,
                    character: 42,
                },
            }
            .into(),
        })];

        assert_eq!(parsed, expected);

        let text = "Test text test text [link](<path/to/link.md#heading>)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDHeadingLink(
            ReferenceData {
                reference_text: "path/to/link#heading".into(),
                display_text: Some("link".into()),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 20,
                    },
                    end: Position {
                        line: 0,
                        character: 53,
                    },
                }
                .into(),
            },
            "path/to/link".into(),
            "heading".into(),
        )];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn md_heading_link_parsing() {
        let text = "Test text test text [link](path/to/link#heading)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDHeadingLink(
            ReferenceData {
                reference_text: "path/to/link#heading".into(),
                display_text: Some("link".into()),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 20,
                    },
                    end: Position {
                        line: 0,
                        character: 48,
                    },
                }
                .into(),
            },
            "path/to/link".into(),
            "heading".into(),
        )];

        assert_eq!(parsed, expected);

        let text = "Test text test text [link](path/to/link.md#heading)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDHeadingLink(
            ReferenceData {
                reference_text: "path/to/link#heading".into(),
                display_text: Some("link".into()),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 20,
                    },
                    end: Position {
                        line: 0,
                        character: 51,
                    },
                }
                .into(),
            },
            "path/to/link".into(),
            "heading".into(),
        )];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn md_block_link_parsing() {
        let text = "Test text test text [link](path/to/link#^index1)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDIndexedBlockLink(
            ReferenceData {
                reference_text: "path/to/link#^index1".into(),
                display_text: Some("link".into()),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 20,
                    },
                    end: Position {
                        line: 0,
                        character: 48,
                    },
                }
                .into(),
            },
            "path/to/link".into(),
            "index1".into(),
        )];

        assert_eq!(parsed, expected);

        let text = "Test text test text [link](path/to/link.md#^index1)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDIndexedBlockLink(
            ReferenceData {
                reference_text: "path/to/link#^index1".into(),
                display_text: Some("link".into()),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 20,
                    },
                    end: Position {
                        line: 0,
                        character: 51,
                    },
                }
                .into(),
            },
            "path/to/link".into(),
            "index1".into(),
        )];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn md_link_with_trailing_parentheses_parsing() {
        // [Issue 260](https://github.com/Feel-ix-343/markdown-oxide/issues/260) covers an issue with parentheses on a new line after a mdlink being parsed as another link.

        let text = r#"
            Buggy cross [link](path/to/link#^index1):

            (this causes bug)
        "#;

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDIndexedBlockLink(
            ReferenceData {
                reference_text: "path/to/link#^index1".into(),
                display_text: Some("link".into()),
                range: Range {
                    start: Position {
                        line: 1,
                        character: 24,
                    },
                    end: Position {
                        line: 1,
                        character: 52,
                    },
                }
                .into(),
            },
            "path/to/link".into(),
            "index1".into(),
        )];

        assert_eq!(parsed, expected);
    }

    #[test]
    fn footnote_link_parsing() {
        let text = "This is a footnote[^1]

[^1]: This is not";
        let parsed = Reference::new(text, "test.md").collect_vec();
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
    fn multi_footnote_link_parsing() {
        let text = "This is a footnote[^1][^2][^3]

[^1]: This is not
[^2]: This is not
[^3]: This is not";
        let parsed = Reference::new(text, "test.md").collect_vec();
        let expected = vec![
            Footnote(ReferenceData {
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
            }),
            Footnote(ReferenceData {
                reference_text: "^2".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 22,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 26,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
            Footnote(ReferenceData {
                reference_text: "^3".into(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 26,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 30,
                    },
                }
                .into(),
                ..ReferenceData::default()
            }),
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn link_parsing_with_png() {
        let text = "This is a png [[link.png]] [[link|display.png]]";
        let parsed = Reference::new(text, "test.md").collect_vec();

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

        let parsed: Vec<_> = MDHeading::new(text).collect();

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

        let parsed = MDIndexedBlock::new(text).collect_vec();

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

        assert_eq!(
            refname,
            Some(Refname {
                full_refname: "test".into(),
                path: "test".to_string().into(),
                ..Default::default()
            })
        )
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

        assert_eq!(
            refname,
            Some(Refname {
                full_refname: "test#Test Heading".to_string(),
                path: Some("test".to_string()),
                infile_ref: Some("Test Heading".to_string())
            })
        )
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

        assert_eq!(
            refname,
            Some(Refname {
                full_refname: "test#^12345".into(),
                path: Some("test".into()),
                infile_ref: "^12345".to_string().into()
            })
        )
    }

    #[test]
    fn parsing_special_text() {
        let text = "’’’󰌶 is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![
            WikiFileLink(ReferenceData {
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
            WikiFileLink(ReferenceData {
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
            WikiFileLink(ReferenceData {
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
    fn test_comprehensive_tag_parsing() {
        let text = r##"# This is a heading

This is a #tag and another #tag/subtag

Chinese: #中文标签 #中文/子标签
Japanese: #テストtag #タグ/子タグ
Korean: #한국어 #한글/서브태그

Edge cases:
- Not a tag: word#notag [[link#not a tag]]
- Number start: #7invalid
- Special chars: #-/_/tag
- In quotes: "Text #引用中的标签 text"
- Complex: #MapOfContext/apworld

#正常标签
    "##;
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
                tag_ref: "tag/subtag".into(),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 27,
                    },
                    end: Position {
                        line: 2,
                        character: 38,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "中文标签".into(),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 9,
                    },
                    end: Position {
                        line: 4,
                        character: 14,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "中文/子标签".into(),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 15,
                    },
                    end: Position {
                        line: 4,
                        character: 22,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "テストtag".into(),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 10,
                    },
                    end: Position {
                        line: 5,
                        character: 17,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "タグ/子タグ".into(),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 18,
                    },
                    end: Position {
                        line: 5,
                        character: 25,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "한국어".into(),
                range: Range {
                    start: Position {
                        line: 6,
                        character: 8,
                    },
                    end: Position {
                        line: 6,
                        character: 12,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "한글/서브태그".into(),
                range: Range {
                    start: Position {
                        line: 6,
                        character: 13,
                    },
                    end: Position {
                        line: 6,
                        character: 21,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "-/_/tag".into(),
                range: Range {
                    start: Position {
                        line: 11,
                        character: 17,
                    },
                    end: Position {
                        line: 11,
                        character: 25,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "引用中的标签".into(),
                range: Range {
                    start: Position {
                        line: 12,
                        character: 19,
                    },
                    end: Position {
                        line: 12,
                        character: 26,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "MapOfContext/apworld".into(),
                range: Range {
                    start: Position {
                        line: 13,
                        character: 11,
                    },
                    end: Position {
                        line: 13,
                        character: 32,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "正常标签".into(),
                range: Range {
                    start: Position {
                        line: 15,
                        character: 0,
                    },
                    end: Position {
                        line: 15,
                        character: 5,
                    },
                }
                .into(),
            },
        ];

        let parsed = MDTag::new(text).collect_vec();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_all_quote_types_in_tags() {
        // Test tags with all types of quotes (Chinese and English, single and double)
        let text = r##"
Chinese double quotes: #测试"引号"标签
English double quotes: #test"quotes"tag
English single quotes: #test'quotes'tag  
Curly single quotes: #test'quotes'tag
Mixed quotes: #混合"quotes'测试'标签"
Plain tag: #plaintext
    "##;
        let expected: Vec<MDTag> = vec![
            MDTag {
                tag_ref: "测试\"引号\"标签".into(),
                range: Range {
                    start: Position {
                        line: 1,
                        character: 23,
                    },
                    end: Position {
                        line: 1,
                        character: 32,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "test\"quotes\"tag".into(),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 23,
                    },
                    end: Position {
                        line: 2,
                        character: 39,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "test'quotes'tag".into(),
                range: Range {
                    start: Position {
                        line: 3,
                        character: 23,
                    },
                    end: Position {
                        line: 3,
                        character: 39,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "test'quotes'tag".into(),
                range: Range {
                    start: Position {
                        line: 4,
                        character: 21,
                    },
                    end: Position {
                        line: 4,
                        character: 37,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "混合\"quotes'测试'标签\"".into(),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 14,
                    },
                    end: Position {
                        line: 5,
                        character: 31,
                    },
                }
                .into(),
            },
            MDTag {
                tag_ref: "plaintext".into(),
                range: Range {
                    start: Position {
                        line: 6,
                        character: 11,
                    },
                    end: Position {
                        line: 6,
                        character: 21,
                    },
                }
                .into(),
            },
        ];
        let parsed = MDTag::new(text).collect_vec();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_obsidian_footnote() {
        let text = "[^1]: This is a footnote";
        let parsed = MDFootnote::new(text).collect_vec();
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
        let parsed = MDFootnote::new(text).collect_vec();
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

    #[test]
    fn parse_link_ref_def() {
        let text = "[ab]: ohreally";

        let parsed = MDLinkReferenceDefinition::new(text).collect_vec();

        let expected = vec![MDLinkReferenceDefinition {
            link_ref_name: "ab".into(),
            url: "ohreally".into(),
            title: None,
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 14,
                },
            }
            .into(),
        }];

        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_link_ref() {
        let text = "This is a [link]j\n\n[link]: linktext";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::LinkRef(ReferenceData {
            reference_text: "link".into(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 10,
                },
                end: Position {
                    line: 0,
                    character: 16,
                },
            }
            .into(),
            ..ReferenceData::default()
        })];

        assert_eq!(parsed, expected);
    }
    #[test]
    fn parse_url_encoded_link() {
        let text = " [f](file%20with%20spaces)";

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDFileLink(ReferenceData {
            reference_text: "file with spaces".into(),
            display_text: Some("f".into()),
            range: Range {
                start: Position {
                    line: 0,
                    character: 1,
                },
                end: Position {
                    line: 0,
                    character: 26,
                },
            }
            .into(),
        })];

        assert_eq!(parsed, expected);
    }
    #[test]
    fn parse_weird_url_encoded_file_link() {
        let text = "[f](%D1%84%D0%B0%D0%B9%D0%BB%20with%20%C3%A9mojis%20%F0%9F%9A%80%20%26%20symbols%20%21%23%40%24%25%26%2A%28%29%2B%3D%7B%7D%7C%5C%22%5C%5C%3A%3B%3F)".into();

        let parsed = Reference::new(text, "test.md").collect_vec();

        let expected = vec![Reference::MDFileLink(ReferenceData {
            reference_text: r##"файл with émojis 🚀 & symbols !#@$%&*()+={}|\"\\:;?"##.into(),
            display_text: Some("f".into()),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 147,
                },
            }
            .into(),
        })];

        assert_eq!(parsed, expected);
    }
}
