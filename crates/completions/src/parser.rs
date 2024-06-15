use std::{ops::Range, path::Path};

use regex::Regex;
use vault::Vault;

use crate::Location;

pub(crate) struct Parser<'a> {
    memfs: &'a dyn ParserMemfs,
}

impl Parser<'_> {
    pub(crate) fn parse_link(&self, location: Location) -> Option<(NamedEntityQuery, QueryInfo)> {
        let line_string = self.memfs.select_line_str(location.path, location.line)?;

        let (link_query, char_range, info) = parse_link_line(line_string, location.character)?;

        Some((
            link_query,
            QueryInfo {
                char_range,
                line: location.line,
                query_syntax_info: info,
            },
        ))
    }
}

#[derive(Debug)]
enum ParsedLinkType {
    Closed,
    Unclosed,
}
#[derive(Debug, PartialEq)]
enum SyntaxType {
    Markdown,
    Wiki,
}
fn parse_link_line(
    line_string: &str,
    character: usize,
) -> Option<(NamedEntityQuery, Range<usize>, QuerySyntaxInfo)> {
    let link_char = r"[^\[\]\(\)]";
    let query_re = format!(
        r"(?<file_ref>{link_char}*?)(#((\^(?<index>{link_char}*?))|(?<heading>{link_char}*?)))??"
    );

    let wiki_re_with_closing = Regex::new(&format!(
        r"\[\[{query_re}(\|(?<display>{link_char}*?))?\]\]"
    ))
    .expect("Regex failed to compile");

    // TODO: consider supporting display text without closing? When would this ever happen??
    let wiki_re_without_closing =
        Regex::new(&format!(r"\[\[{query_re}$")).expect("Regex failed to compile");

    let md_re_with_closing = Regex::new(&format!(r"\[(?<display>{link_char}*?)\]\({query_re}\)"))
        .expect("Regex failed to compile");

    let md_re_without_closing = Regex::new(&format!(r"\[(?<display>{link_char}*?)\]\({query_re}$"))
        .expect("Regex failed to compile");

    let (c, link_type, syntax_type) = wiki_re_with_closing
        .captures_iter(line_string)
        .find(|c| c.get(0).is_some_and(|m| m.range().contains(&character)))
        .map(|c| (c, ParsedLinkType::Closed, SyntaxType::Wiki))
        .or_else(|| {
            wiki_re_without_closing
                .captures_iter(&line_string[..character])
                .find(|c| c.get(0).is_some_and(|m| m.range().start < character))
                .map(|c| (c, ParsedLinkType::Unclosed, SyntaxType::Wiki))
        })
        .or_else(|| {
            md_re_with_closing
                .captures_iter(line_string)
                .find(|c| c.get(0).is_some_and(|m| m.range().contains(&character)))
                .map(|c| (c, ParsedLinkType::Closed, SyntaxType::Markdown))
        })
        .or_else(|| {
            md_re_without_closing
                .captures_iter(&line_string[..character])
                .find(|c| c.get(0).is_some_and(|m| m.range().start < character))
                .map(|c| (c, ParsedLinkType::Unclosed, SyntaxType::Markdown))
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
        .map(|m| NamedEntityInfileQuery::Heading(m.as_str()))
        .or_else(|| {
            c.name("index")
                .map(|m| NamedEntityInfileQuery::Index(m.as_str()))
        });
    let display = c.name("display").map(|m| m.as_str());

    Some((
        NamedEntityQuery {
            file_query: file_ref,
            infile_query: infile_ref,
        },
        char_range,
        QuerySyntaxInfo {
            syntax_type_info: match syntax_type {
                SyntaxType::Wiki => QuerySyntaxTypeInfo::Wiki { display },
                SyntaxType::Markdown => QuerySyntaxTypeInfo::Markdown {
                    display: display.expect("that the display should not be none on markdown link"),
                },
            },
        },
    ))
}

#[derive(Debug, PartialEq)]
pub struct NamedEntityQuery<'a> {
    pub file_query: &'a str,
    pub infile_query: Option<NamedEntityInfileQuery<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum NamedEntityInfileQuery<'a> {
    /// Can be empty excludes the #
    Heading(&'a str),
    /// Can be empty; excludes the ^
    Index(&'a str),
}

pub struct QueryInfo<'fs> {
    pub line: usize,
    pub char_range: Range<usize>,
    pub query_syntax_info: QuerySyntaxInfo<'fs>,
}

pub struct QuerySyntaxInfo<'fs> {
    /// Display: If None, there is no display syntax entered; If Some, this is a structure for it
    /// but the string could be empty; for example [[file#heading|]] or even [](file#heaing)
    pub syntax_type_info: QuerySyntaxTypeInfo<'fs>,
}

impl QuerySyntaxInfo<'_> {
    pub fn display(&self) -> Option<&str> {
        match self.syntax_type_info {
            QuerySyntaxTypeInfo::Markdown { display } => Some(display),
            QuerySyntaxTypeInfo::Wiki { display } => display,
        }
    }
}

/// This is a plain enum for now, but there may be item specific syntax used. For example, if file
/// extensions are used or if paths are used
#[derive(Debug, PartialEq)]
pub enum QuerySyntaxTypeInfo<'a> {
    Markdown { display: &'a str },
    Wiki { display: Option<&'a str> },
}

// NOTE: Enables mocking for tests and provides a slight benefit of decoupling Parser from vault as
// memfs -- which will eventually be replaced by a true MemFS crate.
trait ParserMemfs: Send + Sync {
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

#[cfg(test)]
mod completion_parser_tests {
    use crate::parser::{
        parse_link_line, NamedEntityInfileQuery, NamedEntityQuery, QuerySyntaxTypeInfo,
    };

    #[test]
    fn test_file() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file]] jfkdlsa fjdkl ";

        let (parsed, range, ..) = parse_link_line(line, 55 - 21).unwrap();

        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: None
            }
        );

        assert_eq!(range, 51 - 21..59 - 21)
    }

    #[test]
    fn test_infile_ref_heading() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#heading]] jfkdlsa fjdkl ";

        let (parsed, ..) = parse_link_line(line, 58 - 19).unwrap();

        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: Some(NamedEntityInfileQuery::Heading("heading"))
            }
        )
    }

    #[test]
    fn test_infile_ref_index() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^index]] fjdlkf jsdakl";

        let (parsed, ..) = parse_link_line(line, 58 - 19).unwrap();

        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: Some(NamedEntityInfileQuery::Index("index"))
            }
        )
    }

    #[test]
    fn test_blank_infile_index() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^]]";

        let (parsed, ..) = parse_link_line(line, 58 - 19).unwrap();

        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: Some(NamedEntityInfileQuery::Index(""))
            }
        )
    }

    #[test]
    fn test_blank_infile_heading() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#]]";

        let (parsed, ..) = parse_link_line(line, 58 - 22).unwrap();

        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: Some(NamedEntityInfileQuery::Heading(""))
            }
        )
    }

    #[test]
    fn test_no_closing() {
        //                                                         C
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[this is a query jf dkljfa ";

        let (parsed, ..) = parse_link_line(line, 68 - 21).unwrap();

        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "this is a query",
                infile_query: None
            }
        )
    }

    #[test]
    fn test_markdown_link() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [this is a query](file) jfkdlsa fjdkl ";
        let (parsed, range, info) = parse_link_line(line, 53 - 21).unwrap();

        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: None
            }
        );

        assert_eq!(range, 51 - 21..74 - 21);
        assert_eq!(
            info.syntax_type_info,
            QuerySyntaxTypeInfo::Markdown {
                display: "this is a query"
            }
        );
    }

    #[test]
    fn test_markdown_link_no_closing() {
        //                                                                      C
        let line = "fjlfjdl fjkl lkjfkld fklasj   [this is a query](file jfkldas fjklsd jfkls";
        let (parsed, range, info) = parse_link_line(line, 81 - 21).unwrap();
        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file jfkldas",
                infile_query: None
            }
        );
        assert_eq!(range, 51 - 21..81 - 21);
        assert_eq!(
            info.syntax_type_info,
            QuerySyntaxTypeInfo::Markdown {
                display: "this is a query"
            }
        );
    }

    #[test]
    fn test_markdown_closed_infile_query() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [this is a query](file#heading) jfkdlsa fjdkl ";
        let (parsed, range, info) = parse_link_line(line, 63 - 21).unwrap();
        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: Some(NamedEntityInfileQuery::Heading("heading"))
            }
        );
        assert_eq!(range, 51 - 21..82 - 21);
        assert_eq!(
            info.syntax_type_info,
            QuerySyntaxTypeInfo::Markdown {
                display: "this is a query"
            }
        );
    }

    #[test]
    fn test_markdown_closed_infile_query_index() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [this is a query](file#^index) jfkdlsa fjdkl ";
        let (parsed, range, info) = parse_link_line(line, 63 - 21).unwrap();
        assert_eq!(
            parsed,
            NamedEntityQuery {
                file_query: "file",
                infile_query: Some(NamedEntityInfileQuery::Index("index"))
            }
        );
        assert_eq!(range, 51 - 21..81 - 21);
        assert_eq!(
            info.syntax_type_info,
            QuerySyntaxTypeInfo::Markdown {
                display: "this is a query"
            }
        );
    }

    #[test]
    fn markdown_syntax_display_text() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [](file#^index) jfkdlsa fjdkl ";
        let (_parsed, _range, info) = parse_link_line(line, 63 - 21).unwrap();
        assert_eq!(info.display(), Some(""))
    }

    #[test]
    fn wiki_syntax_display_text_none() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^index|]] jfkdlsa fjdkl ";
        let (_parsed, _range, info) = parse_link_line(line, 63 - 21).unwrap();
        assert_eq!(info.display(), Some(""))
    }

    #[test]
    fn wiki_syntax_display_text_some() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^index|some]] jfkdlsa fjdkl ";
        let (_parsed, _range, info) = parse_link_line(line, 63 - 21).unwrap();
        assert_eq!(info.display(), Some("some"))
    }

    #[test]
    fn wiki_unclosed_with_multiple_links() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file query jfkdlsa fjdkl [[file#^index|some]]";
        let (parsed, _range, _info) = parse_link_line(line, 71 - 21).unwrap();
        assert_eq!(parsed.file_query, "file query jfkdlsa")
    }

    #[test]
    fn wiki_unclosed_after_link() {
        let line = "fjlfjdl fjkl lkjfkld [[link]] fklasj   [[file query jfkdlsa fjdkl";
        let (parsed, _range, _info) = parse_link_line(line, 72 - 21).unwrap();
        assert_eq!(parsed.file_query, "file query")
    }

    #[test]
    fn md_unclosed_before_link() {
        let line = "fjlfjdl fjkl lkjfkld [display](file query f sdklafjdkl  j[another linke](file)";
        let (parsed, _range, info) = parse_link_line(line, 62 - 21).unwrap();
        assert_eq!(parsed.file_query, "file query");
        assert_eq!(info.display(), Some("display"))
    }

    #[test]
    fn md_unclosed_after_link() {
        let line = "fjlfjdl fjkl lkjfkld [display](file) f sdklafjdkl [another](fjsdklf dsjkl fdj asklfsdjklf ";
        let (parsed, _range, info) = parse_link_line(line, 94 - 21).unwrap();
        assert_eq!(parsed.file_query, "fjsdklf dsjkl");
        assert_eq!(info.display(), Some("another"))
    }

    #[test]
    fn wiki_unclosed_with_special_chars() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file query # heading with a # in it and a ^ ajfkl dfkld jlk";
        let (parsed, _, _) = parse_link_line(line, 102 - 21).unwrap();
        assert_eq!(parsed.file_query, "file query ");
        assert_eq!(
            parsed.infile_query,
            Some(NamedEntityInfileQuery::Heading(
                " heading with a # in it and a ^ ajfkl"
            ))
        )
    }
}
