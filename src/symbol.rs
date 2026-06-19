use std::{iter, path::Path};

use itertools::Itertools;
use nucleo_matcher::{
    pattern::{self, Normalization},
    Matcher,
};
use tower_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, SymbolInformation, SymbolKind,
    WorkspaceSymbolParams,
};

use crate::vault::{MDHeading, Vault};

fn compute_match_score(
    matcher: &mut Matcher,
    pattern: &pattern::Pattern,
    symbol: SymbolInformation,
) -> (u32, SymbolInformation) {
    let mut buf = Vec::new();
    (
        match pattern.score(
            nucleo_matcher::Utf32Str::new(symbol.name.as_str(), &mut buf),
            matcher,
        ) {
            None => 0,
            Some(score) => score,
        },
        symbol,
    )
}

pub fn workspace_symbol(
    vault: &Vault,
    _params: &WorkspaceSymbolParams,
) -> Option<Vec<SymbolInformation>> {
    let symbols = vault
        .select_referenceable_nodes(None)
        .into_iter()
        .flat_map(|referenceable| vault.to_symbol_informations(&referenceable))
        .collect_vec();

    // Some clients (e.g. one-shot workspace symbol pickers) send an empty query first and
    // expect the full symbol list so they can handle filtering on their side.
    if _params.query.trim().is_empty() {
        return Some(symbols);
    }

    // Initialize the fuzzy matcher
    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let pattern = pattern::Pattern::parse(
        &_params.query,
        pattern::CaseMatching::Smart,
        Normalization::Smart,
    );

    // Collect symbols (including aliases) and order by fuzzy matching score
    Some(
        symbols
            .into_iter()
            // Fuzzy matcher - compute match score
            .map(|symbol| compute_match_score(&mut matcher, &pattern, symbol))
            // Remove all items with no matches
            .filter(|(score, _)| *score > 0)
            // Sort by match score descending
            .sorted_by(|(a, _), (b, _)| Ord::cmp(b, a))
            // Strip the score from the result
            .map(|(_score, symbol)| symbol)
            .collect_vec(),
    )
}

pub fn document_symbol(
    vault: &Vault,
    _params: &DocumentSymbolParams,
    path: &Path,
) -> Option<DocumentSymbolResponse> {
    let headings = vault.select_headings(path)?;

    let tree = construct_tree(headings)?;
    let lsp = map_to_lsp_tree(tree);

    Some(DocumentSymbolResponse::Nested(lsp))
}

#[derive(PartialEq, Debug)]
struct Node {
    heading: MDHeading,
    children: Option<Vec<Node>>,
}

fn construct_tree(headings: &[MDHeading]) -> Option<Vec<Node>> {
    match &headings {
        [only] => {
            let node = Node {
                heading: only.clone(),
                children: None,
            };
            Some(vec![node])
        }
        [first, rest @ ..] => {
            let break_index = rest
                .iter()
                .find_position(|heading| first.level >= heading.level);

            match break_index.map(|(index, _)| (&rest[..index], &rest[index..])) {
                Some((to_next, rest)) => {
                    // to_next is could be an empty list and rest has at least one item
                    let node = Node {
                        heading: first.clone(),
                        children: construct_tree(to_next), // if to_next is empty, this will return none
                    };

                    Some(
                        iter::once(node)
                            .chain(construct_tree(rest).into_iter().flatten())
                            .collect(),
                    )
                }
                None => {
                    let node = Node {
                        heading: first.clone(),
                        children: construct_tree(rest),
                    };
                    Some(vec![node])
                }
            }
        }
        [] => None,
    }
}

#[allow(deprecated)] // field deprecated has been deprecated in favor of using tags and will be removed in the future
fn map_to_lsp_tree(tree: Vec<Node>) -> Vec<DocumentSymbol> {
    tree.into_iter()
        .map(|node| DocumentSymbol {
            name: node.heading.heading_text,
            kind: SymbolKind::STRUCT,
            deprecated: None,
            tags: None,
            range: *node.heading.range,
            detail: None,
            selection_range: *node.heading.range,
            children: node.children.map(map_to_lsp_tree),
        })
        .collect()
}

#[cfg(test)]
mod test {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::config::{Case, EmbeddedBlockTransclusionLength, Settings};
    use crate::{
        symbol,
        vault::{HeadingLevel, MDHeading, Vault},
    };
    use tower_lsp::lsp_types::WorkspaceSymbolParams;

    fn test_settings() -> Settings {
        Settings {
            dailynote: "%Y-%m-%d".to_string(),
            new_file_folder_path: String::new(),
            daily_notes_folder: String::new(),
            heading_completions: true,
            title_headings: true,
            unresolved_diagnostics: true,
            semantic_tokens: true,
            tags_in_codeblocks: false,
            references_in_codeblocks: false,
            include_md_extension_md_link: false,
            include_md_extension_wikilink: false,
            hover: true,
            case_matching: Case::Smart,
            inlay_hints: true,
            block_transclusion: true,
            block_transclusion_length: EmbeddedBlockTransclusionLength::Full,
            link_filenames_only: false,
            excluded_folders: Vec::new(),
            heading_slug: false,
            callout_completions: true,
        }
    }

    fn temp_vault_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "markdown-oxide-workspace-symbol-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp vault dir");
        dir
    }

    #[test]
    fn test_simple_tree() {
        let headings = vec![
            MDHeading {
                level: HeadingLevel(1),
                heading_text: "First".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(2),
                heading_text: "Second".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(3),
                heading_text: "Third".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(2),
                heading_text: "Second".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(1),
                heading_text: "First".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(1),
                heading_text: "First".to_string(),
                range: Default::default(),
            },
        ];

        let tree = super::construct_tree(&headings);

        let expected = vec![
            symbol::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default(),
                },
                children: Some(vec![
                    symbol::Node {
                        heading: MDHeading {
                            level: HeadingLevel(2),
                            heading_text: "Second".to_string(),
                            range: Default::default(),
                        },
                        children: Some(vec![symbol::Node {
                            heading: MDHeading {
                                level: HeadingLevel(3),
                                heading_text: "Third".to_string(),
                                range: Default::default(),
                            },
                            children: None,
                        }]),
                    },
                    symbol::Node {
                        heading: MDHeading {
                            level: HeadingLevel(2),
                            heading_text: "Second".to_string(),
                            range: Default::default(),
                        },
                        children: None,
                    },
                ]),
            },
            symbol::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default(),
                },
                children: None,
            },
            symbol::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default(),
                },
                children: None,
            },
        ];

        assert_eq!(tree, Some(expected))
    }

    #[test]
    fn test_simple_tree_different() {
        let headings = vec![
            MDHeading {
                level: HeadingLevel(1),
                heading_text: "First".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(2),
                heading_text: "Second".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(3),
                heading_text: "Third".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(1),
                heading_text: "First".to_string(),
                range: Default::default(),
            },
            MDHeading {
                level: HeadingLevel(1),
                heading_text: "First".to_string(),
                range: Default::default(),
            },
        ];

        let tree = super::construct_tree(&headings);

        let expected = vec![
            symbol::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default(),
                },
                children: Some(vec![symbol::Node {
                    heading: MDHeading {
                        level: HeadingLevel(2),
                        heading_text: "Second".to_string(),
                        range: Default::default(),
                    },
                    children: Some(vec![symbol::Node {
                        heading: MDHeading {
                            level: HeadingLevel(3),
                            heading_text: "Third".to_string(),
                            range: Default::default(),
                        },
                        children: None,
                    }]),
                }]),
            },
            symbol::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default(),
                },
                children: None,
            },
            symbol::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default(),
                },
                children: None,
            },
        ];

        assert_eq!(tree, Some(expected))
    }

    #[test]
    fn workspace_symbols_include_aliases() {
        let root_dir = temp_vault_dir();
        let note_path = root_dir.join("note.md");
        fs::write(
            &note_path,
            "---\naliases: [alias-note]\n---\n\n# Heading\nBody text\n",
        )
        .expect("write test note");

        let vault = Vault::construct_vault(&test_settings(), &root_dir)
            .expect("construct test vault");

        let params = WorkspaceSymbolParams {
            query: String::new(),
            ..Default::default()
        };

        let symbols = symbol::workspace_symbol(&vault, &params).expect("workspace symbols");
        let names: Vec<_> = symbols.into_iter().map(|symbol| symbol.name).collect();

        assert!(names.contains(&"note".to_string()));
        assert!(names.contains(&"alias-note".to_string()));
    }
}
