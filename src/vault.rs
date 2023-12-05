use std::{collections::HashMap, path::{Path, PathBuf}, ops::Range};

use itertools::Itertools;
use once_cell::sync::Lazy;
use pathdiff::diff_paths;
use regex::Regex;
use ropey::Rope;
use tower_lsp::lsp_types::Position;

pub fn construct_vault(root_dir: &Path) -> Result<Vault, std::io::Error> {

    let md_file_paths = root_dir
        .read_dir()?
        .filter_map(|f| Result::ok(f))
        .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
        .collect_vec();

    let md_files: HashMap<PathBuf, MDFile> = md_file_paths
        .into_iter()
        .flat_map(|p| {
            let text = std::fs::read_to_string(p.path())?;
            let rope = Rope::from_str(&text);
            let md_file = parse_obsidian_md(&rope);

            return Ok::<(PathBuf, MDFile), std::io::Error>((p.path(), md_file))
        })
        .collect();

    return Ok(Vault {
        files: md_files,
        root_dir: root_dir.into()
    })
}

pub fn reconstruct_vault(old: &mut Vault, new_file: (&Path, &str)) {
    let new_md_file = parse_obsidian_md(&Rope::from_str(new_file.1));
    let new = old.files.get_mut(new_file.0);

    match new {
        Some(file) => { *file = new_md_file; }
        None => { old.files.insert(new_file.0.into(), new_md_file); }
    };
}

fn parse_obsidian_md(rope: &Rope) -> MDFile {

    let links = parse_obsidian_links(rope);
    let headings = parse_obsidian_headings(rope);
    let indexed_blocks = parse_obsidian_indexed_blocks(rope);

    return MDFile { links, headings, indexed_blocks }
}

fn parse_obsidian_links(rope: &Rope) -> Vec<Link> {
    static LINK_RE: Lazy<Regex> = Lazy::new(|| 
        Regex::new(r"\[\[(?<referencetext>[.[^\[\]\|]]+)(\|(?<display>[.[^\[\]|]]+))?\]\]").unwrap()
    ); // A [[link]] that does not have any [ or ] in it

    let links: Vec<Link> = LINK_RE.captures_iter(&rope.to_string())
        .flat_map(|capture| match (capture.get(0), capture.name("referencetext"), capture.name("display")) {
            (Some(full), Some(reference_text), display) => Some((full, reference_text, display)),
            _ => None
        })
        .map(|(outer, re_match, display)| {
        Link {
            reference_text: re_match.as_str().into(),
            range: range_to_position(rope, outer.range()),
            display_text: display.map(|d| d.as_str().into())
        }})
        .collect_vec();

    return links
}

fn parse_obsidian_headings(rope: &Rope) -> Vec<MDHeading> {

    static HEADING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"#+ (.+)").unwrap());

    let headings: Vec<MDHeading> = HEADING_RE.captures_iter(&rope.to_string())
        .flat_map(|c| match (c.get(0), c.get(1)) {
            (Some(full), Some(text)) => Some((full, text)),
            _ => None
        })
        .map(|(full_heading, heading_match)| {

            return MDHeading {
                heading_text: heading_match.as_str().into(),
                range: range_to_position(rope, full_heading.range())
            }
        })
        .collect_vec();

    return headings
}

fn parse_obsidian_indexed_blocks(rope: &Rope) -> Vec<MDIndexedBlock> {

    static INDEXED_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r".+(\^(?<index>\w+))").unwrap());

    let indexed_blocks: Vec<MDIndexedBlock> = INDEXED_BLOCK_RE.captures_iter(&rope.to_string())
        .flat_map(|c| match (c.get(1), c.name("index")) {
            (Some(full), Some(index)) => Some((full, index)),
            _ => None
        })
        .map(|(full, index)|  {
            MDIndexedBlock {
                index: index.as_str().into(),
                range: range_to_position(rope, full.range())
            }
        })
        .collect_vec();

    return indexed_blocks
} // Make this better identify the full blocks

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
    files: HashMap<PathBuf, MDFile>,
    root_dir: PathBuf,
}

/// Linkable algebreic type that easily allows for new linkable nodes to be added if necessary and everything in it should live the same amount because it is all from vault
/// These will also use the current obsidian syntax to come up with reference names for the linkables. These are the things that are using in links ([[Refname]])
pub enum Linkable<'a> {
    MDFile(&'a PathBuf, &'a MDFile),
    Heading(&'a PathBuf, &'a MDHeading),
    IndexedBlock(&'a PathBuf, &'a MDIndexedBlock)
}

fn get_obsidian_ref_path(root_dir: &Path, path: &Path) -> Option<String> {
    diff_paths(path, root_dir).and_then(|diff| diff.with_extension("").to_str().map(|refname| String::from(refname)))
}

impl Linkable<'_> {
    pub fn get_refname(&self, root_dir: &Path) -> Option<String> {
        match self {
            &Linkable::MDFile(path, _) => get_obsidian_ref_path(root_dir, path),
            &Linkable::Heading(path, heading) => get_obsidian_ref_path(root_dir, path).and_then(|refpath| Some(format!("{}#{}", refpath, heading.heading_text))),
            &Linkable::IndexedBlock(path, heading) => get_obsidian_ref_path(root_dir, path).and_then(|refpath| Some(format!("{}#^{}", refpath, heading.index)))
        }
    }

    pub fn get_path(&self) -> &PathBuf {
        match self {
            &Linkable::MDFile(path, _) => path,
            &Linkable::Heading(path, _) => path,
            &Linkable::IndexedBlock(path, _) => path
        }
    }

    pub fn get_range(&self) -> tower_lsp::lsp_types::Range {
        match self {
            &Linkable::MDFile(_, _) => tower_lsp::lsp_types::Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 1 } },
            &Linkable::Heading(_, heading) => heading.range,
            &Linkable::IndexedBlock(_, indexed_block) => indexed_block.range
        }
    }
}

impl Vault {
    /// Select all links ([[Link]]) in a file
    pub fn select_links_in_file(&self, path: &Path) -> Option<&Vec<Link>> {
        self.files.get(path).map(|md| &md.links)
    }

    /// Select all linkable positions in the vault
    pub fn select_linkable_nodes<'a>(&'a self) -> Vec<Linkable<'a>> {
        let files = self.files.iter()
            .map(|(path, md)| Linkable::MDFile(path, md));

        let headings = self.files.iter()
            .flat_map(|(path, md)| md.headings.iter().map(move |h| (path, h)))
            .map(|(path, h)| Linkable::Heading(path, h));

        let indexed_blocks = self.files.iter()
            .flat_map(|(path, md)| md.indexed_blocks.iter().map(move |ib| (path, ib)))
            .map(|(path, ib)| Linkable::IndexedBlock(path, ib));

        return files.into_iter().chain(headings).into_iter().chain(indexed_blocks).collect()
    }

    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MDFile {
    links: Vec<Link>,
    headings: Vec<MDHeading>,
    indexed_blocks: Vec<MDIndexedBlock>
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Link {
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

// tests
#[cfg(test)]
mod vault_tests {
    use std::path::{Path, PathBuf};

    use ropey::Rope;
    use tower_lsp::lsp_types::{Position, Range, Location, Url};

    use crate::{vault::parse_obsidian_headings, gotodef::goto_definition};

    use super::{Link, parse_obsidian_links, MDHeading, parse_obsidian_indexed_blocks, MDIndexedBlock, Linkable, MDFile, construct_vault};

    #[test]
    fn link_parsing() {
        let text = "This is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = parse_obsidian_links(&Rope::from_str(text));

        let expected = vec![
            Link {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 18 } },
            ..Link::default()
            },
            Link {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 19 }, end: tower_lsp::lsp_types::Position { line: 0, character: 29} },
            ..Link::default()
            },
            Link {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 10 } },
            ..Link::default()
            }
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn link_parsin_with_display_text() {
        let text = "This is a [[link|but called different]] [[link 2|222]]\n[[link 3|333]]";
        let parsed = parse_obsidian_links(&Rope::from_str(text));

        let expected = vec![
            Link {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 39 } },
            display_text: Some("but called different".into()),
            },
            Link {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 40 }, end: tower_lsp::lsp_types::Position { line: 0, character: 54} },
            display_text: Some("222".into()),
            },
            Link {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 14 } },
            display_text: Some("333".into()),
            }
        ];

        assert_eq!(parsed, expected)
    }


    #[test]
    fn heading_parsing() {

        let text = r"# This is a heading

Some more text on the second line

Some text under it

some mroe text

more text


## This shoudl be a heading!";

        let parsed = parse_obsidian_headings(&Rope::from_str(text));

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

        let parsed = parse_obsidian_indexed_blocks(&Rope::from_str(text));

        assert_eq!(parsed[0].index, "12345")
    }

    #[test]
    fn test_linkable_reference() {
        let path = Path::new("/home/vault/test.md");
        let path_buf = path.to_path_buf();
        let md_file = MDFile{indexed_blocks: vec![], headings: vec![], links: vec![]};
        let linkable: Linkable = Linkable::MDFile(&path_buf, &md_file);

        let root_dir = Path::new("/home/vault");
        let refname = linkable.get_refname(root_dir);

        assert_eq!(refname, Some("test".into()))
    }

    #[test]
    fn test_linkable_reference_heading() {
        let path = Path::new("/home/vault/test.md");
        let path_buf = path.to_path_buf();
        let md_heading = MDHeading{heading_text: "Test Heading".into(), range: tower_lsp::lsp_types::Range::default()};
        let linkable: Linkable = Linkable::Heading(&path_buf, &md_heading);

        let root_dir = Path::new("/home/vault");
        let refname = linkable.get_refname(root_dir);

        assert_eq!(refname, Some("test#Test Heading".into()))

    }


    #[test]
    fn test_linkable_reference_indexed_block() {
        let path = Path::new("/home/vault/test.md");
        let path_buf = path.to_path_buf();
        let md_indexed_block = MDIndexedBlock{index: "12345".into(), range: tower_lsp::lsp_types::Range::default()};
        let linkable: Linkable = Linkable::IndexedBlock(&path_buf, &md_indexed_block);

        let root_dir = Path::new("/home/vault");
        let refname = linkable.get_refname(root_dir);

        assert_eq!(refname, Some("test#^12345".into()))
    }


    #[test]
    fn parsing_special_text() {
        let text = "’’’󰌶 is a [[link]] [[link 2]]\n[[link 3]]";
        let parsed = parse_obsidian_links(&Rope::from_str(text));

        let expected = vec![
            Link {
            reference_text: "link".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 10 }, end: tower_lsp::lsp_types::Position { line: 0, character: 18 } },
            ..Link::default()
            },
            Link {
            reference_text: "link 2".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 0, character: 19 }, end: tower_lsp::lsp_types::Position { line: 0, character: 29} },
            ..Link::default()
            },
            Link {
            reference_text: "link 3".into(),
            range: tower_lsp::lsp_types::Range { start: tower_lsp::lsp_types::Position { line: 1, character: 0 }, end: tower_lsp::lsp_types::Position { line: 1, character: 10 } },
            ..Link::default()
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
        let proper = Some(Location{
            uri: Url::from_file_path(result_path.to_str().unwrap()).unwrap(),
            range: Range { 
            start: Position { line: 0, character: 0 }, 
            end: Position { line: 0, character: 1 }
            }
        });
        assert_eq!(result, proper);

        let position = Position {line: 6, character: 27};
        let mut path = root_dir.clone();
        path.push("Test.md");
        let result = goto_definition(&vault, position, &path);

        let mut result_path = root_dir.clone();
        result_path.push("Another Test.md");
        let proper = Some(Location{
            uri: Url::from_file_path(result_path.to_str().unwrap()).unwrap(),
            range: Range { 
                start: Position { line: 2, character: 0 }, 
                end: Position { line: 2, character: 24 }
            }
        });
        assert_eq!(result, proper);
    }
}
