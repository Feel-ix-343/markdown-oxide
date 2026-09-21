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

    use tower_lsp::lsp_types::{ClientCapabilities, SymbolKind, WorkspaceSymbolParams};

    use crate::{
        config::Settings,
        symbol,
        vault::{HeadingLevel, MDHeading, Vault},
    };

    fn workspace_params(query: &str) -> WorkspaceSymbolParams {
        WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }
    }

    fn temp_vault_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "markdown-oxide-ws-alias-{}-{}-{}",
            label,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&root).expect("create temp vault");
        root
    }

    /// Regression for #263: aliases from YAML frontmatter must appear in
    /// workspace/symbol results for both empty-query clients (Telescope /
    /// vim.lsp.buf.workspace_symbol) and filtered alias queries. Alias
    /// entries should share the file URI, use SymbolKind::FILE, and expose
    /// the canonical note name via container_name for picker context.
    #[test]
    fn workspace_symbols_include_frontmatter_aliases_with_container() {
        let root = temp_vault_root("aliases");
        fs::write(
            root.join("Canonical Note.md"),
            "---\naliases:\n  - Moon Alias\n  - LaunchPlan\n  - Canonical Note\n  - \" \"\n---\n\n# Heading\n",
        )
        .expect("write aliased note");
        fs::write(root.join("Plain Note.md"), "# Just a heading\n").expect("write plain note");

        let settings = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        let vault = Vault::construct_vault(&settings, &root).unwrap();

        let all = super::workspace_symbol(&vault, &workspace_params(""))
            .expect("empty query should return symbols");

        let canonical = all
            .iter()
            .find(|s| s.name == "Canonical Note")
            .expect("canonical file symbol");
        assert_eq!(canonical.kind, SymbolKind::FILE);
        assert_eq!(canonical.container_name, None);

        let moon = all
            .iter()
            .find(|s| s.name == "Moon Alias")
            .expect("Moon Alias should be listed");
        assert_eq!(moon.kind, SymbolKind::FILE);
        assert_eq!(moon.container_name.as_deref(), Some("Canonical Note"));
        assert_eq!(moon.location.uri, canonical.location.uri);
        assert_eq!(moon.location.range, canonical.location.range);

        assert!(all.iter().any(|s| s.name == "LaunchPlan"));
        // Alias equal to the vault name and blank aliases must not duplicate.
        assert_eq!(
            all.iter().filter(|s| s.name == "Canonical Note").count(),
            1
        );

        let plain = all
            .iter()
            .find(|s| s.name == "Plain Note")
            .expect("unaliased file still appears");
        assert_eq!(plain.container_name, None);

        let by_alias = super::workspace_symbol(&vault, &workspace_params("LaunchPlan"))
            .expect("alias query");
        assert!(by_alias.iter().any(|s| s.name == "LaunchPlan"));

        let by_fuzzy = super::workspace_symbol(&vault, &workspace_params("moon"))
            .expect("fuzzy alias query");
        assert!(by_fuzzy.iter().any(|s| s.name == "Moon Alias"));

        let by_file = super::workspace_symbol(&vault, &workspace_params("Canonical Note"))
            .expect("filename query");
        assert!(by_file.iter().any(|s| s.name == "Canonical Note"));

        fs::remove_dir_all(root).ok();
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
}
