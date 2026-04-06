use once_cell::sync::Lazy;
use regex::Regex;
use ropey::Rope;

use super::{MyRange, Rangeable};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MDCodeBlock {
    range: MyRange,
}

impl MDCodeBlock {
    #[cfg(test)]
    pub fn new(text: &str) -> impl Iterator<Item = MDCodeBlock> + '_ {
        let rope = Rope::from_str(text);
        Self::collect_with_rope(text, &rope).into_iter()
    }

    pub fn collect_with_rope(text: &str, rope: &Rope) -> Vec<MDCodeBlock> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(^|\n)(?<fullblock>``` *(?<lang>[^\n]+)?\n(?<code>(\n|.)*?)\n```)")
                .expect("Codeblock Regex Not Constructing")
        });

        let captures = RE.captures_iter(text);

        static SHORT_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?<fullblock>`[^`\n]+?`)")
                .expect("Short code-block Regex Not Constructing")
        });

        let short_captures = SHORT_RE.captures_iter(text);

        captures.chain(short_captures).flat_map(|captures| {
            Some(MDCodeBlock {
                range: MyRange::from_range(rope, captures.name("fullblock")?.range()),
            })
        }).collect()
    }
}

impl Rangeable for MDCodeBlock {
    fn range(&self) -> &MyRange {
        &self.range
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use tower_lsp::lsp_types::{Position, Range};

    use super::MDCodeBlock;

    #[test]
    fn test_code_block_parsing() {
        let test = r"```python
# Comment

x = 5
```";

        let parsed = MDCodeBlock::new(test).collect_vec();

        let expected = vec![MDCodeBlock {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 4,
                    character: 3,
                },
            }
            .into(),
        }];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_code_block_parsing_later_in_file() {
        let test = r"



```python
# Comment

x = 5
```


fj aklfjd 

";

        let parsed = MDCodeBlock::new(test).collect_vec();

        let expected = vec![MDCodeBlock {
            range: Range {
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 8,
                    character: 3,
                },
            }
            .into(),
        }];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_multiple_codeblocks() {
        let test = r"



```python
# Comment

x = 5
```



```python
# Comment

x = 5
```


fj aklfjd 

";

        let parsed = MDCodeBlock::new(test).collect_vec();

        let expected = vec![
            MDCodeBlock {
                range: Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 8,
                        character: 3,
                    },
                }
                .into(),
            },
            MDCodeBlock {
                range: Range {
                    start: Position {
                        line: 12,
                        character: 0,
                    },
                    end: Position {
                        line: 16,
                        character: 3,
                    },
                }
                .into(),
            },
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_multiple_codeblocks_odd() {
        let test = r"



```
# Comment

x = 5
```



``` python another
# Comment

x = 5
```


fj aklfjd 

";

        let parsed = MDCodeBlock::new(test).collect_vec();

        let expected = vec![
            MDCodeBlock {
                range: Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 8,
                        character: 3,
                    },
                }
                .into(),
            },
            MDCodeBlock {
                range: Range {
                    start: Position {
                        line: 12,
                        character: 0,
                    },
                    end: Position {
                        line: 16,
                        character: 3,
                    },
                }
                .into(),
            },
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_short_code_block_parsing() {
        let test = r" fjdlf jdlk  `test code block` jfkl dlk j";

        let parsed = MDCodeBlock::new(test).collect_vec();

        let expected = vec![MDCodeBlock {
            range: Range {
                start: Position {
                    line: 0,
                    character: 13,
                },
                end: Position {
                    line: 0,
                    character: 30,
                },
            }
            .into(),
        }];

        assert_eq!(parsed, expected)
    }

    #[test]
    fn test_short_code_block_parsing_multiple() {
        let test = r" fjdlf jdlk  `test code block` jfkl `dlk` j";

        let parsed = MDCodeBlock::new(test).collect_vec();

        let expected = vec![
            MDCodeBlock {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 13,
                    },
                    end: Position {
                        line: 0,
                        character: 30,
                    },
                }
                .into(),
            },
            MDCodeBlock {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 36,
                    },
                    end: Position {
                        line: 0,
                        character: 41,
                    },
                }
                .into(),
            },
        ];

        assert_eq!(parsed, expected)
    }

    #[test]
    #[ignore]
    fn test_inline_code_block_perf_regression() {
        let repeated = "`inline` plain text ".repeat(20_000);
        let test = format!("{repeated}\n```rust\nfn main() {{}}\n```\n{repeated}");

        let start = std::time::Instant::now();
        let parsed = MDCodeBlock::new(&test).collect_vec();
        let elapsed = start.elapsed();

        assert_eq!(parsed.len(), 40_001);
        assert!(elapsed < std::time::Duration::from_secs(3), "parsing took {elapsed:?}");
    }
}
