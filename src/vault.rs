use std::{collections::HashMap, path::{Path, PathBuf}, ops::Range};

use itertools::Itertools;
use once_cell::sync::Lazy;
use pathdiff::diff_paths;
use rayon::prelude::*;
use regex::Regex;
use ropey::Rope;
use tower_lsp::lsp_types::{Position};
use walkdir::WalkDir;

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
            let md_file = parse_obsidian_md(&text);

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
    let new_md_file = parse_obsidian_md(new_file.1);
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

fn parse_obsidian_md(text: &str) -> MDFile {

    let links = parse_obsidian_references(text);
    let headings = parse_obsidian_headings(text);
    let indexed_blocks = parse_obsidian_indexed_blocks(text);
    let tags = parse_obsidian_tags(text);

    return MDFile { references: links, headings, indexed_blocks, tags }
}

/// Parse out the references to linkables in each file. This will have links to files and tags
fn parse_obsidian_references(text: &str) -> Vec<Reference> {
    static LINK_RE: Lazy<Regex> = Lazy::new(|| 
        Regex::new(r"\[\[(?<referencetext>[^\[\]\|\.]+)(\|(?<display>[^\[\]\.\|]+))?\]\]").unwrap()
    ); // A [[link]] that does not have any [ or ] in it

    let links: Vec<Reference> = LINK_RE.captures_iter(text)
        .flat_map(|capture| match (capture.get(0), capture.name("referencetext"), capture.name("display")) {
            (Some(full), Some(reference_text), display) => Some((full, reference_text, display)),
            _ => None
        })
        .map(|(outer, re_match, display)| {
        Reference {
            reference_text: re_match.as_str().into(),
            range: range_to_position(&Rope::from_str(text), outer.range()),
            display_text: display.map(|d| d.as_str().into())
        }})
        .collect_vec();

    let tags: Vec<Reference> = parse_obsidian_tags(text).iter().map(|tag| Reference {display_text: None, range: tag.range, reference_text: format!("#{}", tag.tag_ref)}).collect();

    return links.into_iter().chain(tags.into_iter()).collect_vec()
}

fn parse_obsidian_headings(text: &str) -> Vec<MDHeading> {

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

fn parse_obsidian_indexed_blocks(text: &str) -> Vec<MDIndexedBlock> {

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

fn parse_obsidian_tags(text: &str) -> Vec<MDTag> {
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



#[derive(Debug, PartialEq, Eq)]
/// The in memory representation of the obsidian vault files. This data is exposed through an interface of methods to select the vaults data.
/// These methods do not do any interpretation or analysis of the data. That is up to the consumer of this struct. The methods are analogous to selecting on a database. 
pub struct Vault {
    md_files: HashMap<PathBuf, MDFile>,
    ropes: HashMap<PathBuf, Rope>,
    root_dir: PathBuf,
}

#[derive(Debug, Clone)]
/// Linkable algebreic type that easily allows for new linkable nodes to be added if necessary and everything in it should live the same amount because it is all from vault
/// These will also use the current obsidian syntax to come up with reference names for the linkables. These are the things that are using in links ([[Refname]])
pub enum Referenceable<'a> {
    File(&'a PathBuf, &'a MDFile),
    Heading(&'a PathBuf, &'a MDHeading),
    IndexedBlock(&'a PathBuf, &'a MDIndexedBlock),
    Tag(&'a PathBuf, &'a MDTag)
}

fn get_obsidian_ref_path(root_dir: &Path, path: &Path) -> Option<String> {
    diff_paths(path, root_dir).and_then(|diff| diff.with_extension("").to_str().map(|refname| String::from(refname)))
}

impl Referenceable<'_> {
    /// Gets the default reference name for a referenceable. If comparing to a reference text, use the is_reference function
    pub fn get_refname(&self, root_dir: &Path) -> Option<String> {
        match self {
            &Referenceable::File(path, _) => get_obsidian_ref_path(root_dir, path),
            &Referenceable::Heading(path, heading) => get_obsidian_ref_path(root_dir, path).and_then(|refpath| Some(format!("{}#{}", refpath, heading.heading_text))),
            &Referenceable::IndexedBlock(path, heading) => get_obsidian_ref_path(root_dir, path).and_then(|refpath| Some(format!("{}#^{}", refpath, heading.index))),
            &Referenceable::Tag(_, tag) => Some(format!("#{}", tag.tag_ref))
        }
    }

    pub fn is_reference(&self, root_dir: &Path, reference: &Reference) -> bool {
        let text = &reference.reference_text;
        match self {
            &Referenceable::Tag(_, _) => self.get_refname(root_dir).is_some_and(|refname| text.starts_with(&refname)),
            _ => self.get_refname(root_dir) == Some(text.to_string())
        }
    }

    pub fn get_path(&self) -> &PathBuf {
        match self {
            &Referenceable::File(path, _) => path,
            &Referenceable::Heading(path, _) => path,
            &Referenceable::IndexedBlock(path, _) => path,
            &Referenceable::Tag(path, _) => path
        }
    }

    pub fn get_range(&self) -> tower_lsp::lsp_types::Range {
        match self {
            &Referenceable::File(_, _) => tower_lsp::lsp_types::Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 1} },
            &Referenceable::Heading(_, heading) => heading.range,
            &Referenceable::IndexedBlock(_, indexed_block) => indexed_block.range,
            &Referenceable::Tag(_, tag) => tag.range
        }
    }
}

impl Vault {
    /// Select all references ([[link]] or #tag) in a file if path is some, else select all references in the vault. 
    pub fn select_references<'a>(&'a self, path: Option<&'a Path>) -> Option<Vec<(&'a Path, &'a Reference)>> {
        match path {
            Some(path) => self.md_files.get(path).map(|md| &md.references).map(|vec| vec.iter().map(|i| (path, i)).collect()),
            None => Some(self.md_files.iter().map(|(path, md)| md.references.iter().map(|link| (path.as_path(), link))).flatten().collect())
        }
        
    } // TODO: less cloning?

    /// Select all linkable positions in the vault
    pub fn select_linkable_nodes<'a>(&'a self) -> Vec<Referenceable<'a>> {
        let files = self.md_files.iter()
            .map(|(path, md)| Referenceable::File(path, md));

        let headings = self.md_files.iter()
            .flat_map(|(path, md)| md.headings.iter().map(move |h| (path, h)))
            .map(|(path, h)| Referenceable::Heading(path, h));

        let indexed_blocks = self.md_files.iter()
            .flat_map(|(path, md)| md.indexed_blocks.iter().map(move |ib| (path, ib)))
            .map(|(path, ib)| Referenceable::IndexedBlock(path, ib));

        let tags = self.md_files.iter()
            .flat_map(|(path, md)| md.tags.iter().map(move |tag| (path, tag)))
            .map(|(path, tag)| Referenceable::Tag(path, tag));

        return files.into_iter().chain(headings).chain(indexed_blocks).chain(tags).collect()
    }

    pub fn select_linkable_nodes_for_path<'a>(&'a self, path: &'a PathBuf) -> Option<Vec<Referenceable<'a>>> {
        let file = self.md_files.get(path)?;
        let file_linkable = Referenceable::File(path, file);

        let headings = file.headings.iter().map(|h| Referenceable::Heading(path, h));

        let indexed_blocks = file.indexed_blocks.iter().map(|ib| Referenceable::IndexedBlock(path, ib));

        let tags = file.tags.iter().map(|tag| Referenceable::Tag(path, tag));

        return Some(vec![file_linkable].into_iter().chain(headings).chain(indexed_blocks).chain(tags).collect())
    } // TODO: Fix this design. Duplication is bad and I want to require that all references for all types of Referenceable are searched for. 

    pub fn select_line(&self, path: &PathBuf, line: usize) -> Option<Vec<char>> {
        let rope = self.ropes.get(path)?;

        rope.get_line(line).and_then(|slice| Some(slice.chars().collect_vec()))
    }

    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct MDFile {
    references: Vec<Reference>,
    headings: Vec<MDHeading>,
    indexed_blocks: Vec<MDIndexedBlock>,
    tags: Vec<MDTag>
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Reference {
    pub reference_text: String,
    pub display_text: Option<String>,
    pub range: tower_lsp::lsp_types::Range
}

#[derive(Debug, PartialEq, Eq)]
pub struct MDHeading {
    heading_text: String,
    range: tower_lsp::lsp_types::Range
}

#[derive(Debug, PartialEq, Eq)]
pub struct MDIndexedBlock {
    index: String,
    range: tower_lsp::lsp_types::Range
}


#[derive(Debug, PartialEq, Eq)]
pub struct MDTag {
    tag_ref: String,
    range: tower_lsp::lsp_types::Range
}

// tests
#[cfg(test)]
mod vault_tests {
    use std::path::{Path, PathBuf};

    use tower_lsp::lsp_types::{Position, Range, Location, Url};

    use crate::{vault::{parse_obsidian_headings, parse_obsidian_tags}, gotodef::goto_definition};

    use super::{Reference, parse_obsidian_references, MDHeading, parse_obsidian_indexed_blocks, MDIndexedBlock, Referenceable, MDFile, construct_vault, MDTag};

    #[test]
    fn link_parsing() {
        let text = "This is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = parse_obsidian_references(text);

        let expected = vec![
            Reference {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 18 } },
            ..Reference::default()
            },
            Reference {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 19 }, end: tower_lsp::lsp_types::Position { line: 0, character: 29} },
            ..Reference::default()
            },
            Reference {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 10 } },
            ..Reference::default()
            }
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn link_parsin_with_display_text() {
        let text = "This is a [[link|but called different]] [[link 2|222]]\n[[link 3|333]]";
        let parsed = parse_obsidian_references(text);

        let expected = vec![
            Reference {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 39 } },
            display_text: Some("but called different".into()),
            },
            Reference {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 40 }, end: tower_lsp::lsp_types::Position { line: 0, character: 54} },
            display_text: Some("222".into()),
            },
            Reference {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 14 } },
            display_text: Some("333".into()),
            }
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn link_parsing_with_png() {
        let text = "This is a png [[link.png]] [[link|display.png]]";
        let parsed = parse_obsidian_references(text);

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

        let parsed = parse_obsidian_headings(text);

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

        let parsed = parse_obsidian_indexed_blocks(text);

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
        let parsed = parse_obsidian_references(text);

        let expected = vec![
            Reference {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 18 } },
            ..Reference::default()
            },
            Reference {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 19 }, end: tower_lsp::lsp_types::Position { line: 0, character: 29} },
            ..Reference::default()
            },
            Reference {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 10 } },
            ..Reference::default()
            }
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_construct_vault () {
        // get this projects root dir
        let mut root_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).into();
        root_dir.push("TestFiles");

        match construct_vault(&root_dir) {
            Ok(_) => (),
            Err(e) => panic!("{}", e)
        }
    }


    #[test]
    fn test_construct_goto_def () {
        // get this projects root dir
        let mut root_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).into();
        root_dir.push("TestFiles");

        let vault = construct_vault(&root_dir).unwrap();

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

        let parsed = parse_obsidian_tags(&text);

        assert_eq!(parsed, expected)
    }
}

