use std::{
    ops::{Range, RangeBounds},
    path::Path,
};

use regex::Regex;
use vault::Vault;

use crate::Location;

pub(crate) struct Parser<'a> {
    memfs: &'a dyn ParserMemfs,
}

// NOTE: Enables mocking for tests and provides a slight benefit of decoupling Parser from vault as memfs -- which will eventually be replaced by a true MemFS crate.
trait ParserMemfs {
    fn select_line_str(&self, path: &Path, line: usize) -> Option<&str>;
}

impl ParserMemfs for Vault {
    fn select_line_str(&self, path: &Path, line: usize) -> Option<&str> {
        self.select_line_str(path, line)
    }
}

impl<'a> Parser<'a> {
    pub(crate) fn new(vault: &'a Vault) -> Self {
        Self {
            memfs: vault as &dyn ParserMemfs,
        }
    }
}

impl Parser<'_> {
    pub(crate) fn parse_link(&self, location: Location) -> Option<(LinkQuery, LinkInfo)> {
        let line_string = self.memfs.select_line_str(location.path, location.line)?;

        let (link_query, char_range) = parse_link_line(line_string, location.character)?;

        Some((
            link_query,
            LinkInfo {
                char_range,
                line: location.line,
            },
        ))
    }
}

enum ParsedLinkType {
    Closed,
    Unclosed,
}
fn parse_link_line(line_string: &str, character: usize) -> Option<(LinkQuery, Range<usize>)> {
    let re_with_closing =
        Regex::new(r"\[\[(?<file_ref>.*?)(#((\^(?<index>.*?))|(?<heading>.*?)))?\]\]")
            .expect("Regex failed to compile");

    let re_without_closing =
        Regex::new(r"\[\[(?<file_ref>.*?)(#((\^(?<index>.*?))|(?<heading>.*?)))?$")
            .expect("Regex failed to compile");

    let (c, link_type) = re_with_closing
        .captures_iter(line_string)
        .find(|c| c.get(0).is_some_and(|m| m.range().contains(&character)))
        .map(|c| (c, ParsedLinkType::Closed))
        .or_else(|| {
            re_without_closing
                .captures_iter(&line_string[..character])
                .find(|c| c.get(0).is_some_and(|m| m.range().start < character))
                .map(|c| (c, ParsedLinkType::Unclosed))
        })?;

    let char_range = c.get(0)?.range().start..(match link_type {
        ParsedLinkType::Closed => c.get(0)?.range().end,
        ParsedLinkType::Unclosed => character, // this should be correct because the character is one
                                               // beyond the last character typed, so it is the exclusive
                                               // range
    });

    let file_ref = c.name("file_ref")?.as_str();
    let infile_ref = c
        .name("heading")
        .map(|m| InfileRef::Heading(m.as_str()))
        .or_else(|| c.name("index").map(|m| InfileRef::Index(m.as_str())));

    Some((
        LinkQuery {
            file_ref,
            infile_ref,
        },
        char_range,
    ))
}

#[derive(Debug, PartialEq)]
pub struct LinkQuery<'a> {
    pub file_ref: &'a str,
    pub infile_ref: Option<InfileRef<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum InfileRef<'a> {
    /// Can be empty excludes the #
    Heading(&'a str),
    /// Can be empty; excludes the ^
    Index(&'a str),
}

pub(crate) type FileRef<'a> = &'a str;
pub struct LinkInfo {
    pub line: usize,
    pub char_range: Range<usize>,
}

#[cfg(test)]
mod completion_parser_tests {
    use std::{path::PathBuf, str::FromStr};

    use crate::parser::{parse_link_line, InfileRef, LinkQuery};

    use super::{Parser, ParserMemfs};

    #[test]
    fn test_file() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file]] jfkdlsa fjdkl ";

        let (parsed, range) = parse_link_line(line, 55 - 21).unwrap();

        assert_eq!(
            parsed,
            LinkQuery {
                file_ref: "file",
                infile_ref: None
            }
        );

        assert_eq!(range, 51 - 21..59 - 21)
    }

    #[test]
    fn test_infile_ref_heading() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#heading]] jfkdlsa fjdkl ";

        let (parsed, _) = parse_link_line(line, 58 - 19).unwrap();

        assert_eq!(
            parsed,
            LinkQuery {
                file_ref: "file",
                infile_ref: Some(InfileRef::Heading("heading"))
            }
        )
    }

    #[test]
    fn test_infile_ref_index() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^index]] fjdlkf jsdakl";

        let (parsed, _) = parse_link_line(line, 58 - 19).unwrap();

        assert_eq!(
            parsed,
            LinkQuery {
                file_ref: "file",
                infile_ref: Some(InfileRef::Index("index"))
            }
        )
    }

    #[test]
    fn test_blank_infile_index() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^]]";

        let (parsed, _) = parse_link_line(line, 58 - 19).unwrap();

        assert_eq!(
            parsed,
            LinkQuery {
                file_ref: "file",
                infile_ref: Some(InfileRef::Index(""))
            }
        )
    }

    #[test]
    fn test_blank_infile_heading() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#]]";

        let (parsed, _) = parse_link_line(line, 58 - 22).unwrap();

        assert_eq!(
            parsed,
            LinkQuery {
                file_ref: "file",
                infile_ref: Some(InfileRef::Heading(""))
            }
        )
    }

    #[test]
    fn test_no_closing() {
        //                                                         C
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[this is a query jf dkljfa ";

        let (parsed, _) = parse_link_line(line, 68 - 21).unwrap();

        assert_eq!(
            parsed,
            LinkQuery {
                file_ref: "this is a query",
                infile_ref: None
            }
        )
    }
}
