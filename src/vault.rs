use std::{collections::HashMap, path::{Path, PathBuf}, ops::Range};

use itertools::Itertools;
use once_cell::sync::Lazy;
use pathdiff::diff_paths;
use rayon::prelude::*;
use regex::Regex;
use ropey::Rope;
use tower_lsp::lsp_types::Position;
use walkdir::WalkDir;


#[derive(Debug, PartialEq, Eq)]
/// The in memory representation of the obsidian vault files. This data is exposed through an interface of methods to select the vaults data.
/// These methods do not do any interpretation or analysis of the data. That is up to the consumer of this struct. The methods are analogous to selecting on a database. 
pub struct Vault {
    md_files: HashMap<PathBuf, MDFile>,
    ropes: HashMap<PathBuf, Rope>,
    root_dir: PathBuf,
}


impl Vault {
    pub fn construct_vault(root_dir: &Path) -> Result<Vault, std::io::Error> {

        let md_file_paths = WalkDir::new(root_dir)
            .into_iter()
            .filter_entry(|e| !e.file_name().to_str().map(|s| s.starts_with(".")).unwrap_or(false))
            .flatten()
            .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
            .collect_vec();

        let md_files: HashMap<PathBuf, MDFile> = md_file_paths
            .par_iter()
            .flat_map(|p| {
                let text = std::fs::read_to_string(p.path())?;
                let md_file = MDFile::new(&text);

                return Ok::<(PathBuf, MDFile), std::io::Error>((p.path().into(), md_file))
            })
            .collect();

        let ropes = md_file_paths
            .par_iter()
            .flat_map(|p| {
                let text = std::fs::read_to_string(p.path())?;
                let rope = Rope::from_str(&text);

                return Ok::<(PathBuf, Rope), std::io::Error>((p.path().into(), rope))
            })
            .collect();

        return Ok(Vault {
            ropes,
            md_files,
            root_dir: root_dir.into()
        })
    }

    pub fn reconstruct_vault(old: &mut Vault, new_file: (&PathBuf, &str)) {
        let new_md_file = MDFile::new(new_file.1);
        let new = old.md_files.get_mut(new_file.0);

        match new {
            Some(file) => { *file = new_md_file; }
            None => { old.md_files.insert(new_file.0.into(), new_md_file); }
        };

        let new_rope = Rope::from_str(new_file.1);
        let rope_entry = old.ropes.get_mut(new_file.0);

        match rope_entry {
            Some(rope) => { *rope = new_rope; },
            None => { old.ropes.insert(new_file.0.into(), new_rope); }
        }
    }
    /// Select all references ([[link]] or #tag) in a file if path is some, else select all references in the vault.
    pub fn select_references<'a>(&'a self, path: Option<&'a Path>) -> Option<Vec<(&'a Path, &'a Reference)>> {
        match path {
            Some(path) => self.md_files.get(path).map(|md| &md.references).map(|vec| vec.iter().map(|i| (path, i)).collect()),
            None => Some(self.md_files.iter().map(|(path, md)| md.references.iter().map(|link| (path.as_path(), link))).flatten().collect())
        }
        
    } // TODO: less cloning?

    /// Select all linkable positions in the vault
    pub fn select_referenceable_nodes<'a>(&'a self, path: Option<&'a PathBuf>) -> Vec<Referenceable<'a>> {

        let files = match path {
            None => self.md_files.iter().map(|(path, md)| Referenceable::File(path, md)).collect_vec(),
            Some(path) => vec![self.md_files.get(path).map(|md| Referenceable::File(path, md))].into_iter().flatten().collect_vec()
        };

        let headings = self.md_files.iter()
            .flat_map(|(path, md)| md.headings.iter().map(move |h| (path, h)))
            .map(|(path, h)| Referenceable::Heading(path, h));

        let indexed_blocks = self.md_files.iter()
            .flat_map(|(path, md)| md.indexed_blocks.iter().map(move |ib| (path, ib)))
            .map(|(path, ib)| Referenceable::IndexedBlock(path, ib));

        let tags = self.md_files.iter()
            .flat_map(|(path, md)| md.tags.iter().map(move |tag| (path, tag)))
            .map(|(path, tag)| Referenceable::Tag(path, tag));


        let footnotes = self.md_files.iter()
            .flat_map(|(path, md)| md.footnotes.iter().map(move |footnote| (path, footnote)))
            .map(|(path, footnote)| Referenceable::Footnote(path, footnote));

        return files.into_iter().chain(headings).chain(indexed_blocks).chain(tags).chain(footnotes).collect()
    }

    pub fn select_line(&self, path: &Path, line: usize) -> Option<Vec<char>> {
        let rope = self.ropes.get(path)?;

        rope.get_line(line).and_then(|slice| Some(slice.chars().collect_vec()))
    }

    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    pub fn select_referenceable_preview(&self, referenceable: &Referenceable) -> Option<String> {
        match referenceable {
            Referenceable::Footnote(_, _) => {
                let range = referenceable.get_range();
                Some(String::from_iter(self.select_line(&referenceable.get_path(), range.start.line as usize)?))
            },
            Referenceable::Heading(_, _) => {
                let range = referenceable.get_range();
                Some((range.start.line..=range.end.line + 10)
                    .map(|ln| self.select_line(&referenceable.get_path(), ln as usize))
                    .flatten() // flatten those options!
                    .map(|vec| String::from_iter(vec))
                    .join(""))

            },
            Referenceable::IndexedBlock(_, _) => {
                let range = referenceable.get_range();
                self.select_line(&referenceable.get_path(), range.start.line as usize).map(String::from_iter)
            },
            Referenceable::File(_, _) => {
                let file_text = self.ropes.get(referenceable.get_path()).unwrap();
                Some(String::from(file_text))
            }
            _ => return None
        }
    }
}





fn range_to_position(rope: &Rope, range: Range<usize>) -> tower_lsp::lsp_types::Range {
    // convert from byte offset to char offset
    let char_start = rope.byte_to_char(range.start);
    let char_end = rope.byte_to_char(range.end);


    let start_line = rope.char_to_line(char_start);
    let start_offset = char_start - rope.line_to_char(start_line);

    let end_line = rope.char_to_line(char_end);
    let end_offset = char_end - rope.line_to_char(end_line);

    return tower_lsp::lsp_types::Range {
        start: Position {line: start_line as u32,character: start_offset as u32},
        end: Position {line: end_line as u32, character: end_offset as u32}
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct MDFile {
    references: Vec<Reference>,
    headings: Vec<MDHeading>,
    indexed_blocks: Vec<MDIndexedBlock>,
    tags: Vec<MDTag>,
    footnotes: Vec<MDFootnote>
}

impl MDFile {
    fn new(text: &str) -> MDFile {

        let links = Reference::new(text);
        let headings = MDHeading::new(text);
        let indexed_blocks = MDIndexedBlock::new(text);
        let tags = MDTag::new(text);
        let footnotes = MDFootnote::new(text);

        return MDFile { references: links, headings, indexed_blocks, tags, footnotes }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct ReferenceData {
    pub reference_text: String,
    pub display_text: Option<String>,
    pub range: tower_lsp::lsp_types::Range
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Reference {
    Tag(ReferenceData),
    Link(ReferenceData),
    Footnote(ReferenceData)
}

impl Default for Reference {
    fn default() -> Self {
        Link(ReferenceData::default())
    }
}

use Reference::*;

impl Reference {
    pub fn data(&self) -> &ReferenceData {
        match &self {
            Tag(data) => data,
            Link(data) => data,
            Footnote(data) => data
        }
    }

    pub fn matches_type(&self, other: &Reference) -> bool {
        match &other {
            Tag(_) => matches!(self, Tag(_)),
            Link(_) => matches!(self, Link(_)),
            Footnote(_) => matches!(self, Footnote(_)),
        }
    }

    fn new(text: &str) -> Vec<Reference> {
        static LINK_RE: Lazy<Regex> = Lazy::new(|| 
            Regex::new(r"\[\[(?<referencetext>[^\[\]\|\.]+)(\|(?<display>[^\[\]\.\|]+))?\]\]").unwrap()
        ); // A [[link]] that does not have any [ or ] in it

        let links: Vec<Reference> = LINK_RE.captures_iter(text)
            .flat_map(|capture| match (capture.get(0), capture.name("referencetext"), capture.name("display")) {
                (Some(full), Some(reference_text), display) => Some((full, reference_text, display)),
                _ => None
            })
            .map(|(outer, re_match, display)| {

                return Link(ReferenceData {
                    reference_text: re_match.as_str().into(),
                    range: range_to_position(&Rope::from_str(text), outer.range()),
                    display_text: display.map(|d| d.as_str().into())
                })

            })
            .collect_vec();

        let tags: Vec<Reference> = MDTag::new(text).iter().map(|tag| Tag(ReferenceData {display_text: None, range: tag.range, reference_text: format!("#{}", tag.tag_ref)})).collect();


        static FOOTNOTE_LINK_RE: Lazy<Regex> = Lazy::new(|| 
            Regex::new(r"[^\[](?<full>\[(?<index>\^[^\[\] ]+)\])[^\:]").unwrap()
        );
        let footnote_references: Vec<Reference> = FOOTNOTE_LINK_RE.captures_iter(text)
            .flat_map(|capture| match (capture.name("full"), capture.name("index")) {
                (Some(full), Some(index)) => Some((full, index)),
                _ => None
            })
            .map(|(outer, index)| {
                Footnote(ReferenceData {
                    reference_text: index.as_str().into(),
                    range: range_to_position(&Rope::from_str(text), outer.range()),
                    display_text: None
                })})
            .collect_vec();

        return links.into_iter().chain(tags.into_iter()).chain(footnote_references).collect_vec()
    }
}



#[derive(Debug, PartialEq, Eq)]
pub struct MDHeading {
    heading_text: String,
    range: tower_lsp::lsp_types::Range
}

impl MDHeading {
    fn new(text: &str) -> Vec<MDHeading> {

        static HEADING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"#+ (?<heading_text>.+)").unwrap());

        let headings: Vec<MDHeading> = HEADING_RE.captures_iter(text)
            .flat_map(|c| match (c.get(0), c.name("heading_text")) {
                (Some(full), Some(text)) => Some((full, text)),
                _ => None
            })
            .map(|(full_heading, heading_match)| {

                return MDHeading {
                    heading_text: heading_match.as_str().trim_end().into(),
                    range: range_to_position(&Rope::from_str(text), full_heading.range())
                }

            })
            .collect_vec();

        return headings
    }

}

#[derive(Debug, PartialEq, Eq)]
pub struct MDIndexedBlock {
    index: String,
    range: tower_lsp::lsp_types::Range
}

impl MDIndexedBlock {
    fn new(text: &str) -> Vec<MDIndexedBlock> {

        static INDEXED_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r".+ (\^(?<index>\w+))").unwrap());

        let indexed_blocks: Vec<MDIndexedBlock> = INDEXED_BLOCK_RE.captures_iter(&text.to_string())
            .flat_map(|c| match (c.get(1), c.name("index")) {
                (Some(full), Some(index)) => Some((full, index)),
                _ => None
            })
            .map(|(full, index)|  {
                MDIndexedBlock {
                    index: index.as_str().into(),
                    range: range_to_position(&Rope::from_str(text), full.range())
                }
            })
            .collect_vec();

        return indexed_blocks
    } // Make this better identify the full blocks

}


#[derive(Debug, Eq, PartialEq)]
pub struct MDFootnote {
    index: String,
    footnote_text: String,
    range: tower_lsp::lsp_types::Range
}

impl MDFootnote {
    fn new(text: &str) -> Vec<MDFootnote> {
        // static FOOTNOTE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r".+ (\^(?<index>\w+))").unwrap());
        static FOOTNOTE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[(?<index>\^[^ \[\]]+)\]\:(?<text>.+)").unwrap());

        let footnotes: Vec<MDFootnote> = FOOTNOTE_RE.captures_iter(&text.to_string())
            .flat_map(|c| match (c.get(0), c.name("index"), c.name("text")) {
                (Some(full), Some(index), Some(footnote_text)) => Some((full, index, footnote_text)),
                _ => None
            })
            .map(|(full, index, footnote_text)|  {
                MDFootnote {
                    footnote_text: footnote_text.as_str().trim_start().into(),
                    index: index.as_str().into(),
                    range: range_to_position(&Rope::from_str(text), full.range())
                }
            })
            .collect_vec();

        return footnotes
    }
}


#[derive(Debug, PartialEq, Eq)]
pub struct MDTag {
    tag_ref: String,
    range: tower_lsp::lsp_types::Range
}

impl MDTag {
    fn new(text: &str) -> Vec<MDTag> {
        static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\n|\A| )(?<full>#(?<tag>[.[^ \n\#]]+))(\n|\z| )").unwrap());


        let tagged_blocks = TAG_RE.captures_iter(&text.to_string())
            .flat_map(|c| match (c.name("full"), c.name("tag")) {
                (Some(full), Some(index)) => Some((full, index)),
                _ => None
            })
            .filter(|(_, index)| index.as_str().chars().any(|c| c.is_alphabetic()))
            .map(|(full, index)|  {
                MDTag {
                    tag_ref: index.as_str().into(),
                    range: range_to_position(&Rope::from_str(text), full.range())
                }
            })
            .collect_vec();

        return tagged_blocks
    }
}


#[derive(Debug, Clone)]
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
    Footnote(&'a PathBuf, &'a MDFootnote)
}

/// Utility function
fn get_obsidian_ref_path(root_dir: &Path, path: &Path) -> Option<String> {
    diff_paths(path, root_dir).and_then(|diff| diff.with_extension("").to_str().map(|refname| String::from(refname)))
}

impl Referenceable<'_> {
    /// Gets the generic reference name for a referenceable. This will not include any display text. If trying to determine if text is a reference of a particular referenceable, use the `is_reference` function
    pub fn get_refname(&self, root_dir: &Path) -> Option<String> {
        match self {
            &Referenceable::File(path, _) => get_obsidian_ref_path(root_dir, path),
            &Referenceable::Heading(path, heading) => get_obsidian_ref_path(root_dir, path).and_then(|refpath| Some(format!("{}#{}", refpath, heading.heading_text))),
            &Referenceable::IndexedBlock(path, heading) => get_obsidian_ref_path(root_dir, path).and_then(|refpath| Some(format!("{}#^{}", refpath, heading.index))),
            &Referenceable::Tag(_, tag) => Some(format!("#{}", tag.tag_ref)),
            &Referenceable::Footnote(_, footnote) => Some(footnote.index.clone()),
        }
    }

    pub fn is_reference(&self, root_dir: &Path, reference: &Reference, file_path: &Path) -> bool {
        let text = &reference.data().reference_text;
        match self {
            &Referenceable::Tag(_, _) => matches!(reference, Tag(_)) && self.get_refname(root_dir).is_some_and(|refname| text.starts_with(&refname)),
            &Referenceable::Footnote(path, _footnote) => matches!(reference, Footnote(_)) && self.get_refname(root_dir).as_ref() == Some(text) && path.as_path() == file_path,
            _ => matches!(reference, Link(_)) && self.get_refname(root_dir) == Some(text.to_string())
        }
    }

    pub fn get_path(&self) -> &Path {
        match self {
            &Referenceable::File(path, _) => path,
            &Referenceable::Heading(path, _) => path,
            &Referenceable::IndexedBlock(path, _) => path,
            &Referenceable::Tag(path, _) => path,
            &Referenceable::Footnote(path, _) => path
        }
    }

    pub fn get_range(&self) -> tower_lsp::lsp_types::Range {
        match self {
            &Referenceable::File(_, _) => tower_lsp::lsp_types::Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 1} },
            &Referenceable::Heading(_, heading) => heading.range,
            &Referenceable::IndexedBlock(_, indexed_block) => indexed_block.range,
            &Referenceable::Tag(_, tag) => tag.range,
            &Referenceable::Footnote(_, footnote) => footnote.range
        }
    }
}


// tests
#[cfg(test)]
mod vault_tests {
    use std::path::{Path, PathBuf};

    use tower_lsp::lsp_types::{Position, Range, Location, Url};

    use crate::gotodef::goto_definition;
    use crate::vault::ReferenceData;

    use super::{Reference,  MDHeading,  MDIndexedBlock, Referenceable, MDFile, MDTag, MDFootnote};
    use super::Vault;
    use super::Reference::*;

    #[test]
    fn link_parsing() {
        let text = "This is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = Reference::new(text);

        let expected = vec![
            Link(ReferenceData {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 18 } },
            ..ReferenceData::default()
            }),
            Link(ReferenceData {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 19 }, end: tower_lsp::lsp_types::Position { line: 0, character: 29} },
            ..ReferenceData::default()
            }),
            Link(ReferenceData {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 10 } },
            ..ReferenceData::default()
            })
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn link_parsin_with_display_text() {
        let text = "This is a [[link|but called different]] [[link 2|222]]\n[[link 3|333]]";
        let parsed = Reference::new(text);

        let expected = vec![
            Link(ReferenceData {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 39 } },
            display_text: Some("but called different".into()),
            }),
            Link(ReferenceData {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 40 }, end: tower_lsp::lsp_types::Position { line: 0, character: 54} },
            display_text: Some("222".into()),
            }),
            Link(ReferenceData {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 14 } },
            display_text: Some("333".into()),
            })
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn footnote_link_parsing() {
        let text = "This is a footnote[^1]

[^1]: This is not";
        let parsed = Reference::new(text);
        let expected = vec![
            Footnote(ReferenceData {
            reference_text: "^1".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 18 }, end: tower_lsp::lsp_types::Position { line: 0, character: 22 } },
            ..ReferenceData::default()
            })
        ];

        assert_eq!(parsed,expected)
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
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 0 }, end: tower_lsp::lsp_types::Position { line: 0, character: 19} }
            },
            MDHeading {
            heading_text: "This shoudl be a heading!".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 11, character: 0 }, end: tower_lsp::lsp_types::Position { line: 11, character: 28} }
            }
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
        let md_heading = MDHeading{heading_text: "Test Heading".into(), range: tower_lsp::lsp_types::Range::default()};
        let linkable: Referenceable = Referenceable::Heading(&path_buf, &md_heading);

        let root_dir = Path::new("/home/vault");
        let refname = linkable.get_refname(root_dir);

        assert_eq!(refname, Some("test#Test Heading".into()))

    }


    #[test]
    fn test_linkable_reference_indexed_block() {
        let path = Path::new("/home/vault/test.md");
        let path_buf = path.to_path_buf();
        let md_indexed_block = MDIndexedBlock{index: "12345".into(), range: tower_lsp::lsp_types::Range::default()};
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
            Link(ReferenceData {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 18 } },
            ..ReferenceData::default()
            }),
            Link(ReferenceData {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 19 }, end: tower_lsp::lsp_types::Position { line: 0, character: 29} },
            ..ReferenceData::default()
            }),
            Link(ReferenceData {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 10 } },
            ..ReferenceData::default()
            })
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_construct_vault () {
        // get this projects root dir
        let mut root_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).into();
        root_dir.push("TestFiles");

        match Vault::construct_vault(&root_dir) {
            Ok(_) => (),
            Err(e) => panic!("{}", e)
        }
    }


    #[test]
    fn test_construct_goto_def () {
        // get this projects root dir
        let mut root_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).into();
        root_dir.push("TestFiles");

        let vault = Vault::construct_vault(&root_dir).unwrap();

        let position = Position {line: 5, character: 2};
        let mut path = root_dir.clone();
        path.push("A third test.md");
        let result = goto_definition(&vault, position, &path);

        let mut result_path = root_dir.clone();
        result_path.push("Another Test.md");
        let proper = Some(vec![Location{
            uri: Url::from_file_path(result_path.to_str().unwrap()).unwrap(),
            range: Range { 
            start: Position { line: 0, character: 0 }, 
            end: Position { line: 0, character: 1 }
            }
            }]);
        assert_eq!(result, proper);

        let position = Position {line: 6, character: 27};
        let mut path = root_dir.clone();
        path.push("Test.md");
        let result = goto_definition(&vault, position, &path);

        let mut result_path = root_dir.clone();
        result_path.push("Another Test.md");
        let proper = Some(vec![Location{
            uri: Url::from_file_path(result_path.to_str().unwrap()).unwrap(),
            range: Range { 
            start: Position { line: 2, character: 0 }, 
            end: Position { line: 2, character: 24 }
            }
            }]);
        assert_eq!(result, proper);
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
            start: Position { line: 2, character: 10 },
            end: Position { line: 2, character: 14 }
            }
            },
            MDTag {
            tag_ref: "tag/ttagg".into(),
            range: Range {
            start: Position { line: 4, character: 12 },
            end: Position { line: 4, character: 22 }
            }
            },
            MDTag {
            tag_ref: "MapOfContext/apworld".into(),
            range: Range {
            start: Position { line: 8, character: 0 },
            end: Position { line: 8, character: 21 }
            }
            }

        ];

        let parsed = MDTag::new(&text);

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_obsidian_footnote() {
        let text = "[^1]: This is a footnote";
        let parsed = MDFootnote::new(text);
        let expected = vec![
            MDFootnote {
            index: "^1".into(),
            footnote_text: "This is a footnote".into(),
            range: Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 24 }
            }
            }
        ];

        assert_eq!(parsed, expected);



        let text = 
        r"# This is a heading

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
            start: Position { line: 4, character: 0 },
            end: Position { line: 4, character: 19 }
            }
            },
            MDFootnote {
            index: "^2".into(),
            footnote_text: "Another footnote".into(),
            range: Range {
            start: Position { line: 8, character: 0 },
            end: Position { line: 8, character: 22 }
            }
            },
            MDFootnote {
            index: "^a".into(),
            footnote_text: "Third footnot3".into(),
            range: Range {
            start: Position { line: 9, character: 0 },
            end: Position { line: 9, character: 19 }
            }
            }
        ];

        assert_eq!(parsed, expected)
    }
}

