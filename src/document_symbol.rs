use std::{path::{Path, Iter}, iter};

use itertools::Itertools;
use tower_lsp::lsp_types::{DocumentSymbolParams, DocumentSymbolResponse, SymbolInformation, SymbolKind, Location, DocumentSymbol};

use crate::vault::{Vault, MDHeading};

pub fn document_symbol(vault: &Vault, params: DocumentSymbolParams, path: &Path) -> Option<DocumentSymbolResponse> {

    let headings = vault.select_headings(path)?;

    let tree = construct_tree(&headings);
    let lsp = map_to_lsp_tree(tree);

    return Some(DocumentSymbolResponse::Nested(lsp))
}


#[derive(PartialEq, Debug)]
struct Node {
    heading: MDHeading,
    children: Option<Vec<Node>>
}

fn construct_tree(headings: &[ MDHeading ]) -> Vec<Node> {
    match &headings {
        [first, rest @ ..]  => {

            let break_index = rest.iter().find_position(|heading| first.level >= heading.level);

            match break_index {
                Some((index, _)) if index != 0 => {
                    let to_next = &rest[..index];

                    let node = Node {
                        heading: first.clone(),
                        children: Some(construct_tree(to_next))
                    };

                    return iter::once(node).chain(construct_tree(&rest[index..])).collect()
                },
                None if rest.len() != 0 => {
                    let node = Node {
                        heading: first.clone(),
                        children: Some(construct_tree(rest))
                    };

                    return vec![node]
                },
                Some((_, _)) => { // this is when the index is 0
                    let node = Node {
                        heading: first.clone(),
                        children: None
                    };

                    return iter::once(node).chain(construct_tree(rest)).collect()
                },
                None => {
                    let node = Node {
                        heading: first.clone(),
                        children: None
                    };
                    return vec![node]

                }
            }

        },
        [] => panic!("This should never happen")
    }
}


fn map_to_lsp_tree(tree: Vec<Node>) -> Vec<DocumentSymbol> {
    tree.into_iter()
        .map(|node| {
            DocumentSymbol {
                name: node.heading.heading_text,
                kind: SymbolKind::STRUCT,
                deprecated: None,
                tags: None,
                range: node.heading.range,
                detail: None,
                selection_range: node.heading.range,
                children: node.children.map(|children| {
                    map_to_lsp_tree(children)
                })
            }
        })
        .collect()
}


#[cfg(test)]
mod test {
    use crate::vault::{MDHeading, HeadingLevel};

    #[test]
    fn test_simple_tree() {
        let headings = vec![
            MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(2),
            heading_text: "Second".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(3),
            heading_text: "Third".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(2),
            heading_text: "Second".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
        ];

        let tree = super::construct_tree(&headings);

        let expected = vec![
            super::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default()
                },
                children: Some(vec![
                    super::Node {
                        heading: MDHeading {
                        level: HeadingLevel(2),
                        heading_text: "Second".to_string(),
                        range: Default::default()
                    },
                    children: Some(vec![
                        super::Node {
                            heading: MDHeading {
                            level: HeadingLevel(3),
                            heading_text: "Third".to_string(),
                            range: Default::default()
                        },
                        children: None
                        }
                    ])
                    },
                        super::Node {
                        heading: MDHeading {
                        level: HeadingLevel(2),
                        heading_text: "Second".to_string(),
                        range: Default::default()
                        },
                        children: None
                    }
                ])
            },
            super::Node {
            heading: MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
            children: None
            },
            super::Node {
            heading: MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
            children: None
            }
        ];

        assert_eq!(tree, expected)
    }

    #[test]
    fn test_simple_tree_different() {
        let headings = vec![
            MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(2),
            heading_text: "Second".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(3),
            heading_text: "Third".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
            MDHeading {
            level: HeadingLevel(1),
            heading_text: "First".to_string(),
            range: Default::default()
            },
        ];

        let tree = super::construct_tree(&headings);

        let expected = vec![
            super::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default()
                },
                children: Some(vec![
                    super::Node {
                        heading: MDHeading {
                        level: HeadingLevel(2),
                        heading_text: "Second".to_string(),
                        range: Default::default()
                    },
                    children: Some(vec![
                        super::Node {
                            heading: MDHeading {
                            level: HeadingLevel(3),
                            heading_text: "Third".to_string(),
                            range: Default::default()
                        },
                        children: None
                        }
                    ])
                    },
                ])
            },
            super::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default()
                },
                children: None
            },
            super::Node {
                heading: MDHeading {
                    level: HeadingLevel(1),
                    heading_text: "First".to_string(),
                    range: Default::default()
                },
                children: None
            }
        ];

        assert_eq!(tree, expected)
    }
}
