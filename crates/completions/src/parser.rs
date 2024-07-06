use std::{ops::Range, path::Path};

use md_regex_parser::{MDLinkParser, MDRegexParseable};
use regex::Captures;
use vault::Vault;

use crate::Location;

pub(crate) struct Parser<'a> {
    memfs: &'a dyn ParserMemfs,
}

impl<'a> Parser<'a> {
    pub fn parse_entity_query(
        &self,
        location: Location<'a>,
    ) -> Option<(NamedRefCmdQuery, QueryMetadata)> {
        self.parse_query(location)
    }

    pub fn parse_block_query(
        &self,
        location: Location<'a>,
    ) -> Option<(BlockLinkCmdQuery, QueryMetadata)> {
        self.parse_query(location)
    }

    fn line_string(&self, location: Location) -> Option<&'a str> {
        self.memfs.select_line_str(location.path, location.line)
    }

    fn parse_query<T: MDRegexParseable<'a>>(
        &self,
        location: Location<'a>,
    ) -> Option<(T, QueryMetadata<'a>)> {
        let line_string = self.line_string(location)?;
        let (q, char_range, info) = {
            let character = location.character;
            MDLinkParser::new(line_string, character as usize).parse()
        }?;

        Some((q, QueryMetadata::new(location, char_range, info)))
    }
}

// NOTE: Enables mocking for tests and provides a slight benefit of decoupling Parser from vault as
// memfs -- which will eventually be replaced by a true MemFS crate.
trait ParserMemfs: Send + Sync {
    fn select_line_str(&self, path: &Path, line: u32) -> Option<&str>;
}

impl ParserMemfs for Vault {
    fn select_line_str(&self, path: &Path, line: u32) -> Option<&str> {
        self.select_line_str(path, line as usize)
    }
}

impl<'a> Parser<'a> {
    pub(crate) fn new(vault: &'a Vault) -> Self {
        Self {
            memfs: vault as &dyn ParserMemfs,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct NamedRefCmdQuery<'a> {
    pub file_query: &'a str,
    pub infile_query: Option<EntityInfileQuery<'a>>,
}

impl NamedRefCmdQuery<'_> {
    // NOTE: this is sort or re-implemented by multiple traits with methods meaning the same thing, but centralizing the implementation here
    // prevents duplication
    pub fn grep_string(&self) -> String {
        match &self.infile_query {
            Some(EntityInfileQuery::Heading(h)) => format!("{}#{}", self.file_query, h),
            Some(EntityInfileQuery::Index(i)) => format!("{}#^{}", self.file_query, i),
            None => self.file_query.to_string(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EntityInfileQuery<'a> {
    /// Can be empty excludes the #
    Heading(&'a str),
    /// Can be empty; excludes the ^
    Index(&'a str),
}

#[derive(Debug, PartialEq)]
/// DATA
pub struct BlockLinkCmdQuery<'a> {
    pub grep_string: &'a str,
}

pub struct QueryMetadata<'fs> {
    pub line: u32,
    pub char_range: Range<usize>,
    pub query_syntax_info: QuerySyntaxInfo<'fs>,
    pub path: &'fs Path,
    pub cursor: u32,
}

impl<'fs> QueryMetadata<'fs> {
    pub fn new(
        location: Location<'fs>,
        char_range: Range<usize>,
        info: QuerySyntaxInfo<'fs>,
    ) -> Self {
        Self {
            line: location.line,
            char_range,
            query_syntax_info: info,
            path: location.path,
            cursor: location.character,
        }
    }
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

impl<'a> MDRegexParseable<'a> for NamedRefCmdQuery<'a> {
    fn from_captures(captures: Captures<'a>) -> Option<Self> {
        let file_ref = captures.name("file_ref")?.as_str();
        let infile_ref = captures
            .name("heading")
            .map(|m| EntityInfileQuery::Heading(m.as_str()))
            .or_else(|| {
                captures
                    .name("index")
                    .map(|m| EntityInfileQuery::Index(m.as_str()))
            });

        Some(NamedRefCmdQuery {
            file_query: file_ref,
            infile_query: infile_ref,
        })
    }

    fn associated_regex_constructor(char_class: &str) -> String {
        format!(
            r"(?<file_ref>{char_class}*?)(#((\^(?<index>{char_class}*?))|(?<heading>{char_class}*?)))??"
        )
    }
}

impl<'a> MDRegexParseable<'a> for BlockLinkCmdQuery<'a> {
    fn from_captures(captures: Captures<'a>) -> Option<Self> {
        Some(BlockLinkCmdQuery {
            grep_string: captures.name("grep")?.as_str(),
        })
    }

    fn associated_regex_constructor(char_class: &str) -> String {
        format!(" (?<grep>{char_class}*?)")
    }
}

mod md_regex_parser {
    use std::ops::Range;

    use regex::{Captures, Regex};

    use super::{QuerySyntaxInfo, QuerySyntaxTypeInfo};

    pub struct MDLinkParser<'a> {
        hay: &'a str,
        character: usize,
    }

    pub trait MDRegexParseable<'a>: Sized {
        fn from_captures(captures: Captures<'a>) -> Option<Self>;
        fn associated_regex_constructor(char_class: &str) -> String;
    }

    impl<'a> MDLinkParser<'a> {
        pub fn new(string: &'a str, character: usize) -> MDLinkParser {
            MDLinkParser {
                hay: string,
                character,
            }
        }

        pub fn parse<T: MDRegexParseable<'a>>(
            &self,
        ) -> Option<(T, Range<usize>, QuerySyntaxInfo<'a>)> {
            let link_char = r"[^\[\]\(\)]";

            let query_re = T::associated_regex_constructor(link_char);

            let wiki_re_with_closing = Regex::new(&format!(
                r"\[\[{query_re}(\|(?<display>{link_char}*?))?\]\]"
            ))
            .expect("Regex failed to compile");

            // TODO: consider supporting display text without closing? When would this ever happen??
            let wiki_re_without_closing =
                Regex::new(&format!(r"\[\[{query_re}$")).expect("Regex failed to compile");

            let md_re_with_closing =
                Regex::new(&format!(r"\[(?<display>{link_char}*?)\]\({query_re}\)"))
                    .expect("Regex failed to compile");

            let md_re_without_closing =
                Regex::new(&format!(r"\[(?<display>{link_char}*?)\]\({query_re}$"))
                    .expect("Regex failed to compile");

            let (c, link_type, syntax_type) = wiki_re_with_closing
                .captures_iter(self.hay)
                .find(|c| {
                    c.get(0)
                        .is_some_and(|m| m.range().contains(&self.character))
                })
                .map(|c| (c, ParsedLinkType::Closed, SyntaxType::Wiki))
                .or_else(|| {
                    wiki_re_without_closing
                        .captures_iter(&self.hay[..self.character])
                        .find(|c| c.get(0).is_some_and(|m| m.range().start < self.character))
                        .map(|c| (c, ParsedLinkType::Unclosed, SyntaxType::Wiki))
                })
                .or_else(|| {
                    md_re_with_closing
                        .captures_iter(self.hay)
                        .find(|c| {
                            c.get(0)
                                .is_some_and(|m| m.range().contains(&self.character))
                        })
                        .map(|c| (c, ParsedLinkType::Closed, SyntaxType::Markdown))
                })
                .or_else(|| {
                    md_re_without_closing
                        .captures_iter(&self.hay[..self.character])
                        .find(|c| c.get(0).is_some_and(|m| m.range().start < self.character))
                        .map(|c| (c, ParsedLinkType::Unclosed, SyntaxType::Markdown))
                })?;

            let char_range = c.get(0)?.range().start..(match link_type {
                ParsedLinkType::Closed => c.get(0)?.range().end,
                ParsedLinkType::Unclosed => self.character, // this should be correct because the character is one
                                                            // beyond the last character typed, so it is the exclusive
                                                            // range
            });

            let display = c.name("display").map(|m| m.as_str());

            Some((
                T::from_captures(c)?,
                char_range,
                QuerySyntaxInfo {
                    syntax_type_info: match syntax_type {
                        SyntaxType::Wiki => QuerySyntaxTypeInfo::Wiki { display },
                        SyntaxType::Markdown => QuerySyntaxTypeInfo::Markdown {
                            display: display
                                .expect("that the display should not be none on markdown link"),
                        },
                    },
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
}

#[cfg(test)]
mod named_query_parse_tests {
    use crate::parser::{
        md_regex_parser::MDLinkParser, EntityInfileQuery, NamedRefCmdQuery, QuerySyntaxTypeInfo,
    };

    #[test]
    fn test_file() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file]] jfkdlsa fjdkl ";

        let (parsed, range, ..) = MDLinkParser::new(line, 55 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();

        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "file",
                infile_query: None
            }
        );

        assert_eq!(range, 51 - 21..59 - 21)
    }

    #[test]
    fn test_infile_ref_heading() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#heading]] jfkdlsa fjdkl ";

        let (parsed, ..) = MDLinkParser::new(line, 58 - 19)
            .parse::<NamedRefCmdQuery>()
            .unwrap();

        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "file",
                infile_query: Some(EntityInfileQuery::Heading("heading"))
            }
        )
    }

    #[test]
    fn test_infile_ref_index() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^index]] fjdlkf jsdakl";

        let (parsed, ..) = MDLinkParser::new(line, 58 - 19)
            .parse::<NamedRefCmdQuery>()
            .unwrap();

        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "file",
                infile_query: Some(EntityInfileQuery::Index("index"))
            }
        )
    }

    #[test]
    fn test_blank_infile_index() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^]]";

        let (parsed, ..) = MDLinkParser::new(line, 58 - 19)
            .parse::<NamedRefCmdQuery>()
            .unwrap();

        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "file",
                infile_query: Some(EntityInfileQuery::Index(""))
            }
        )
    }

    #[test]
    fn test_blank_infile_heading() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#]]";

        let (parsed, ..) = MDLinkParser::new(line, 58 - 22)
            .parse::<NamedRefCmdQuery>()
            .unwrap();

        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "file",
                infile_query: Some(EntityInfileQuery::Heading(""))
            }
        )
    }

    #[test]
    fn test_no_closing() {
        //                                                         C
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[this is a query jf dkljfa ";

        let (parsed, ..) = MDLinkParser::new(line, 68 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();

        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "this is a query",
                infile_query: None
            }
        )
    }

    #[test]
    fn test_markdown_link() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [this is a query](file) jfkdlsa fjdkl ";
        let (parsed, range, info) = MDLinkParser::new(line, 53 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();

        assert_eq!(
            parsed,
            NamedRefCmdQuery {
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
        let (parsed, range, info) = MDLinkParser::new(line, 81 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(
            parsed,
            NamedRefCmdQuery {
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
        let (parsed, range, info) = MDLinkParser::new(line, 63 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "file",
                infile_query: Some(EntityInfileQuery::Heading("heading"))
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
        let (parsed, range, info) = MDLinkParser::new(line, 63 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(
            parsed,
            NamedRefCmdQuery {
                file_query: "file",
                infile_query: Some(EntityInfileQuery::Index("index"))
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
        let (_parsed, _range, info) = MDLinkParser::new(line, 63 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(info.display(), Some(""))
    }

    #[test]
    fn wiki_syntax_display_text_none() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^index|]] jfkdlsa fjdkl ";
        let (_parsed, _range, info) = MDLinkParser::new(line, 63 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(info.display(), Some(""))
    }

    #[test]
    fn wiki_syntax_display_text_some() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file#^index|some]] jfkdlsa fjdkl ";
        let (_parsed, _range, info) = MDLinkParser::new(line, 63 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(info.display(), Some("some"))
    }

    #[test]
    fn wiki_unclosed_with_multiple_links() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file query jfkdlsa fjdkl [[file#^index|some]]";
        let (parsed, _range, _info) = MDLinkParser::new(line, 71 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(parsed.file_query, "file query jfkdlsa")
    }

    #[test]
    fn wiki_unclosed_after_link() {
        let line = "fjlfjdl fjkl lkjfkld [[link]] fklasj   [[file query jfkdlsa fjdkl";
        let (parsed, _range, _info) = MDLinkParser::new(line, 72 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(parsed.file_query, "file query")
    }

    #[test]
    fn md_unclosed_before_link() {
        let line = "fjlfjdl fjkl lkjfkld [display](file query f sdklafjdkl  j[another linke](file)";
        let (parsed, _range, info) = MDLinkParser::new(line, 62 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(parsed.file_query, "file query");
        assert_eq!(info.display(), Some("display"))
    }

    #[test]
    fn md_unclosed_after_link() {
        let line = "fjlfjdl fjkl lkjfkld [display](file) f sdklafjdkl [another](fjsdklf dsjkl fdj asklfsdjklf ";
        let (parsed, _range, info) = MDLinkParser::new(line, 94 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(parsed.file_query, "fjsdklf dsjkl");
        assert_eq!(info.display(), Some("another"))
    }

    #[test]
    fn wiki_unclosed_with_special_chars() {
        let line = "fjlfjdl fjkl lkjfkld fklasj   [[file query # heading with a # in it and a ^ ajfkl dfkld jlk";
        let (parsed, _, _) = MDLinkParser::new(line, 102 - 21)
            .parse::<NamedRefCmdQuery>()
            .unwrap();
        assert_eq!(parsed.file_query, "file query ");
        assert_eq!(
            parsed.infile_query,
            Some(EntityInfileQuery::Heading(
                " heading with a # in it and a ^ ajfkl"
            ))
        )
    }
}

#[cfg(test)]
mod unnamed_query_tests {
    use crate::parser::{md_regex_parser::MDLinkParser, BlockLinkCmdQuery};

    #[test]
    fn basic_test() {
        let text = "fjkalf kdsjfkd  [[ fjakl fdjk]] fjdl kf j";
        let (d, _, _) = MDLinkParser::new(text, 50 - 21)
            .parse::<BlockLinkCmdQuery>()
            .unwrap();
        assert_eq!("fjakl fdjk", d.grep_string)
    }

    #[test]
    fn unclosed() {
        let text = "fjkalf kdsjfkd  [[ fjakl fdjk fjdl kf j";
        let (d, _, _) = MDLinkParser::new(text, 50 - 21)
            .parse::<BlockLinkCmdQuery>()
            .unwrap();
        assert_eq!("fjakl fdjk", d.grep_string)
    }

    #[test]
    fn multiple_closed() {
        let text = "fjka[[thisis ]] [[ fjakl fdjk]][[fjk]]j";
        let (d, _, _) = MDLinkParser::new(text, 50 - 21)
            .parse::<BlockLinkCmdQuery>()
            .unwrap();
        assert_eq!("fjakl fdjk", d.grep_string)
    }

    #[test]
    fn multiple_unclosed() {
        let text = "fjka[[thisis ]] [[ fjakl fdjk  jklfd slk [[fjk]]j";
        let (d, _, _) = MDLinkParser::new(text, 50 - 21)
            .parse::<BlockLinkCmdQuery>()
            .unwrap();
        assert_eq!("fjakl fdjk", d.grep_string)
    }

    #[test]
    fn not_unnamed_query() {
        let text = "fjka[[thisis ]] [[fjakl fdjkk]]  jklfd slk [[fjk]]j";
        assert!(MDLinkParser::new(text, 50 - 21)
            .parse::<BlockLinkCmdQuery>()
            .is_none())
    }
}
