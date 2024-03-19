use std::{path::PathBuf, time::SystemTime};

use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit, Position, Range, TextEdit};

use crate::vault::{get_obsidian_ref_path, MDFile, MDHeading, MDIndexedBlock, Reference, Referenceable, Vault};

use super::{matcher::{fuzzy_match, fuzzy_match_completions, Matchable, OrderedCompletion}, Completable, Completer, Context};

/// Range on a single line; assumes that the line number is known. 
type LineRange = std::ops::Range<usize>;

pub struct MarkdownLinkCompleter<'a> {
    /// The display text of a link to be completed
    display: (String, LineRange),
    /// the filepath of the markdown link to be completed
    path: (String, LineRange),
    /// the infile ref; the range is the whole span of the infile ref. (including the ^ for Block refs)
    infile_ref: Option<(PartialInfileRef, LineRange)>,

    partial_link: (String, LineRange),
    full_range: LineRange,
    line_nr: usize,
    file_path: std::path::PathBuf,
    vault: &'a Vault
}

impl<'a> Completer<'a> for MarkdownLinkCompleter<'a> {

    fn construct(context: Context<'a>, path: &std::path::Path, line: usize, character: usize) -> Option<Self>
    where Self: Sized {

        let Context { vault, opened_files: _ } = context;

        let line_chars = vault.select_line(path, line as isize)?;
        let line_to_cursor = line_chars.get(0..character)?;

        static PARTIAL_MDLINK_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\[(?<display>[^\[\]\(\)]*)\]\((?<path>[^\[\]\(\)\#]*)(\#(?<infileref>[^\[\]\(\)]*))?$").unwrap()
        }); // [display](relativePath)

        let line_string = String::from_iter(line_to_cursor);

        let captures = PARTIAL_MDLINK_REGEX.captures(&line_string)?;

        let (full, display, reftext, infileref) = (
            captures.get(0)?,
            captures.name("display")?,
            captures.name("path")?,
            captures.name("infileref"),
        );

        let reference_under_cursor =
        Reference::new(&line_string)
            .into_iter()
            .find(|reference| {
                reference.range.start.character <= character as u32
                && reference.range.end.character >= character as u32
            });

        let full_range = match reference_under_cursor {
            Some( reference @ (Reference::MDFileLink(..)
                | Reference::MDHeadingLink(..)
                | Reference::MDIndexedBlockLink(..)),
            ) => reference.range.start.character as usize..reference.range.end.character as usize,
            None if line_to_cursor.get(character) == Some(&')') => {
                full.range().start..full.range().end + 1
            }
            _ => full.range(),
        };


        let partial_infileref = infileref.map(|infileref| {

            let chars = infileref.as_str().chars().collect::<Vec<char>>();

            let range = infileref.range();

            match chars.as_slice() {
                ['^', rest @ ..] => (PartialInfileRef::BlockRef(String::from_iter(rest)), range),
                [rest @ ..] => (PartialInfileRef::HeadingRef(String::from_iter(rest)), range),
            }

        });

        let partial = Some(MarkdownLinkCompleter {
            path: (reftext.as_str().to_string(), reftext.range()),
            display: (display.as_str().to_string(), display.range()),
            infile_ref: partial_infileref,
            partial_link: (full.as_str().to_string(), full.range()),
            full_range,
            line_nr: line,
            file_path: path.to_path_buf(),
            vault
        });

        partial
    }

    fn completions(&self) -> Vec<impl Completable<'a, MarkdownLinkCompleter<'a>>> {

        let filter_text = format!(
            "{}{}",
            self.path.0,
            self.infile_ref
                .clone()
                .map(|(infile, _)| format!("#{}", infile.completion_string()))
                .unwrap_or("".to_string())
        );


        let referenceables = self.vault.select_referenceable_nodes(None);

        // Get and filter referenceables
        let completions = referenceables
            .into_par_iter()
            .flat_map(|referenceable| LinkCompletion::new(referenceable.clone()))
            .collect::<Vec<_>>();

        let filtered = fuzzy_match_completions(&filter_text, completions);

        filtered

    }
}

#[derive(Debug, Clone)]
enum PartialInfileRef {
    HeadingRef(String),
    /// The partial reference to a block, not including the ^ index
    BlockRef(String)
}

impl PartialInfileRef {
    fn completion_string(&self) -> String {
        match self {
            PartialInfileRef::HeadingRef(s) => s.to_string(),
            PartialInfileRef::BlockRef(s) => format!("^{}", s),
        }
    }
}





#[derive(Debug, Clone)]
enum LinkCompletion<'a> {
    File {
        mdfile: &'a MDFile,
        match_string: String,
    },
    Heading {
        heading: &'a MDHeading,
        match_string: String,
    },
    Block {
        indexed: &'a MDIndexedBlock,
        match_string: String,
    },
    Unresolved {
        match_string: String,
        /// Infile ref includes all after #, including ^
        infile_ref: Option<String>,
    },
}

use LinkCompletion::*;

impl LinkCompletion<'_> {
    fn new<'a>(referenceable: Referenceable<'a>) -> Option<LinkCompletion<'a>> {
        match referenceable {
            Referenceable::File(_, mdfile) => Some(File { mdfile, match_string: mdfile.path.file_stem()?.to_str()?.to_string() }),
            Referenceable::Heading(path, mdheading) => Some(Heading {heading: mdheading, match_string: format!("{}#{}", path.file_stem()?.to_str()?, mdheading.heading_text)}),
            Referenceable::IndexedBlock(path, indexed) => Some(Block{ indexed, match_string: format!("{}#^{}", path.file_stem()?.to_str()?, indexed.index)}),
            Referenceable::UnresovledFile(_, file) => Some(Unresolved { match_string: file.clone(), infile_ref: None }),
            Referenceable::UnresolvedHeading(_, s1, s2) => Some(Unresolved { match_string: format!("{}#{}", s1, s2), infile_ref: Some(s2.clone()) }),
            Referenceable::UnresovledIndexedBlock(_, s1, s2) => Some(Unresolved { match_string: format!("{}#^{}", s1, s2), infile_ref: Some(format!("^{}", s2)) }),
            _ => None
        }
    }
}


impl<'a> Completable<'a, MarkdownLinkCompleter<'a>>  for LinkCompletion<'a> {
    fn completion(&self, markdown_link_completer: &MarkdownLinkCompleter) -> CompletionItem {

        let label = self.match_string();

        let MarkdownLinkCompleter { display, path: _, infile_ref: _, partial_link: _, full_range, line_nr, file_path: _, vault: _ } = markdown_link_completer;

        let link_infile_ref = match self {
            File { mdfile: _, match_string: _ } => None,
            Self::Block { indexed, match_string: _ } => Some(format!("#^{}", indexed.index)),
            Self::Heading { heading, match_string: _ } => Some(format!("#{}", heading.heading_text)),
            Self::Unresolved { match_string: _, infile_ref } => infile_ref.clone()
        };

        let binding = (display.0.as_str(), link_infile_ref);
        let link_display_text = match binding {
            ("", Some(ref infile)) => &infile,
                // Get the first heading of the file, if possible. 
            ("", None) => match self {
                Self::File { mdfile, match_string: _ } => mdfile.headings.get(0).map(|heading| heading.heading_text.as_str()).unwrap_or(""),
                _ => ""
            }
            (display, _) => display,
        };


        let link_ref_text = match label.contains(' ') {
            true => format!("<{}>", label),
            false => label.to_owned()
        };

        let link_text = format!(
            "[${{1:{}}}]({})",
            link_display_text,
            link_ref_text
        );


        let text_edit = CompletionTextEdit::Edit(TextEdit {
            range: Range {
                start: Position {
                    line: *line_nr as u32,
                    character: full_range.start as u32,
                },
                end: Position {
                    line: *line_nr as u32,
                    character: full_range.end as u32,
                },
            },
            new_text: link_text,
        });

        CompletionItem {
            label: label.to_string(),
            kind: Some(match self {
                Self::File { mdfile: _, match_string: _ } => CompletionItemKind::FILE,
                Self::Heading { heading: _, match_string: _ } | Self::Block { indexed: _, match_string: _ } => CompletionItemKind::REFERENCE,
                Self::Unresolved { match_string: _, infile_ref: _ } => CompletionItemKind::KEYWORD
            }),
            label_details: match self {
                Self::Unresolved { match_string: _, infile_ref: _ } => Some(CompletionItemLabelDetails{
                    detail: Some("Unresolved".into()),
                    description: None
                }),
                _ => None
            },
            text_edit: Some(text_edit),
            ..Default::default()
        }

    }
}


impl<'a> Completable<'a, WikiLinkCompleter<'a>> for LinkCompletion<'a> {
    fn completion(&self, completer: &WikiLinkCompleter<'a>) -> CompletionItem {
        todo!()
    }
}


impl Matchable for LinkCompletion<'_> {
    fn match_string(&self) -> &str {
        match self {
            File{mdfile: _, match_string} 
                | Heading { heading: _, match_string }
                | Block { indexed: _, match_string }
                | Unresolved { match_string, infile_ref: _ }
                => &match_string,
        }
    }
}


pub struct WikiLinkCompleter<'a> {
    vault: &'a Vault,
    cmp_text: Vec<char>,
    files: &'a [PathBuf]
}

impl<'a> Completer<'a> for WikiLinkCompleter<'a> {


    fn construct(context: Context<'a>, path: &std::path::Path, line: usize, character: usize) -> Option<Self>
        where Self: Sized {

        let Context { vault, opened_files } = context;

        let line_chars = vault.select_line(path, line as isize)?;

        let index = line_chars.get(0..=character)? // select only the characters up to the cursor
            .iter()
            .enumerate() // attach indexes
            .tuple_windows() // window into pairs of characters
            .collect::<Vec<(_, _)>>()
            .into_iter()
            .rev() // search from the cursor back
            .find(|((_, &c1), (_, &c2))| c1 == '[' && c2 == '[')
            .map(|(_, (i, _))| i); // only take the index; using map because find returns an option

        let index = index.and_then(|index| {
            if line_chars.get(index..character)?.into_iter().contains(&']') {
                None
            } else {
                Some(index)
            }
        });

        index.and_then(|index| {
            let cmp_text = line_chars.get(index+1..character)?;

            Some(WikiLinkCompleter{
                vault,
                cmp_text: cmp_text.to_vec(),
                files: opened_files
            })
        })
    }

    fn completions(&self) -> Vec<impl Completable<'a, Self>> where Self: Sized {
        let WikiLinkCompleter { vault, cmp_text, files } = self;

        match *self.cmp_text {
            // Give recent referenceables; TODO: improve this; 
            [] => {
                files
                    .iter()
                    .map(|path| {
                        match std::fs::metadata(path).and_then(|meta| meta.modified()) {
                            Ok(modified) => (path, modified),
                            Err(_) => (path, SystemTime::UNIX_EPOCH),
                        }
                    })
                    .sorted_by_key(|(_, modified)| *modified)
                    .flat_map(|(path, modified)| {

                        let referenceables = vault.select_referenceable_nodes(Some(&path));

                        let modified_string = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs().to_string();

                        Some(referenceables.into_iter()
                            .flat_map(move |referenceable| Some(
                                OrderedCompletion::<WikiLinkCompleter, LinkCompletion>::new(
                                    LinkCompletion::new(referenceable)?,
                                    modified_string.clone()
                                ))
                            ))

                    })
                    .flatten()
                    .collect_vec()
            },
            ref filter_text @ [..] if !filter_text.contains(&']') => {
                let filter_text = &self.cmp_text;


                let referenceables = self.vault.select_referenceable_nodes(None);

                // Get and filter referenceables
                let completions = referenceables
                    .into_par_iter()
                    .flat_map(|referenceable| LinkCompletion::new(referenceable.clone()))
                    .collect::<Vec<_>>();

                let filtered = fuzzy_match_completions(&String::from_iter(filter_text), completions);

                filtered
            },
            _ => vec![]
        }
    }
}



