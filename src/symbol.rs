use std::{iter, path::Path};

use itertools::Itertools;
use tower_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Location, SymbolInformation,
    SymbolKind, Url, WorkspaceSymbolParams,
};

use crate::vault::{MDHeading, Referenceable, Vault};

pub fn workspace_symbol(
    vault: &Vault,
    _params: &WorkspaceSymbolParams,
) -> Option<Vec<SymbolInformation>> {
    let referenceables = vault.select_referenceable_nodes(None);
    let symbol_informations = referenceables
        .into_iter()
        .filter_map(|referenceable| {
            Some(SymbolInformation {
                name: referenceable.get_refname(vault.root_dir())?,
                kind: match referenceable {
                    Referenceable::File(_, _) => SymbolKind::FILE,
                    Referenceable::Tag(_, _) => SymbolKind::CONSTANT,
                    _ => SymbolKind::KEY,
                },
                location: Location {
                    uri: Url::from_file_path(referenceable.get_path()).ok()?,
                    range: *referenceable.get_range()?,
                },
                container_name: None,
                tags: None,
                deprecated: None,
            })
        })
        .collect_vec();

    Some(symbol_informations)
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
    use crate::{
        symbol,
        vault::{HeadingLevel, MDHeading},
    };

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
