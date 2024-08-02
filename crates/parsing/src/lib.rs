use std::{
    fmt::{Debug, Formatter},
    ops::Not,
    sync::Arc,
};

use ropey::Rope;
use tree_sitter::Range;
use tree_sitter_md::{MarkdownCursor, MarkdownParser, MarkdownTree};

struct Document {
    sections: Vec<Section>,
    rope: Rope,
}

impl Debug for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("sections", &self.sections)
            .finish()
    }
}

impl Document {
    fn new(node: tree_sitter::Node, markdown_tree: &MarkdownTree, text: &str) -> Option<Document> {
        let rope = Rope::from_str(text);
        match node.kind() {
            "document" => {
                let mut cursor = node.walk();
                let children = node.children(&mut cursor);
                let sections = children
                    .flat_map(|it| Section::parse_section(it, markdown_tree, 0, rope.clone()))
                    .collect();
                Some(Document { sections, rope })
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Section {
    heading: Option<Heading>,
    level: usize,
    nodes: Vec<Node>,
}

impl Section {
    fn parse_section(
        node: tree_sitter::Node,
        markdown_tree: &MarkdownTree,
        level: usize,
        rope: Rope,
    ) -> Option<Section> {
        match node.kind() {
            "section" => {
                let mut cursor = node.walk();
                let children = node.children(&mut cursor);
                let heading = node.child(0).and_then(|it| {
                    if it.kind() == "atx_heading" {
                        Heading::parse(it, rope.clone())
                    } else {
                        None
                    }
                });
                let nodes: Vec<Node> = children
                    .flat_map(|node| match node.kind() {
                        "paragraph" => vec![Node::Block(Block::ParagraphBlock(
                            ParagraphBlock::parse(node, rope.clone()),
                        ))],
                        "list" => {
                            let mut cursor = node.walk();
                            let children = node.children(&mut cursor);
                            children
                                .flat_map(|child| match child.kind() {
                                    "list_item" => {
                                        ListBlock::parse(child, markdown_tree, rope.clone())
                                    }
                                    _ => None,
                                })
                                .map(|it| Block::ListBlock(it))
                                .map(|it| Node::Block(it))
                                .collect::<Vec<_>>()
                        }
                        "section" => {
                            Section::parse_section(node, markdown_tree, level + 1, rope.clone())
                                .map(|it| vec![Node::Section(it)])
                                .unwrap_or(Vec::new())
                        } // need my monad trans
                        _ => vec![],
                    })
                    .collect();

                if nodes.is_empty().not() || heading.is_some() {
                    Some(Section {
                        heading,
                        level,
                        nodes,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Node {
    Block(Block),
    Section(Section),
}

#[derive(Debug)]
enum Block {
    ListBlock(ListBlock),
    ParagraphBlock(ParagraphBlock),
}

struct ListBlock {
    full_range: Range,
    text_range: Range,
    text: Arc<str>,
    children: Option<Vec<ListBlock>>,
}

impl Debug for ListBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListBlock")
            .field("text", &self.text)
            .field("children", &self.children)
            .finish()
    }
}

impl ListBlock {
    fn parse(
        node: tree_sitter::Node,
        _markdown_tree: &MarkdownTree,
        rope: Rope,
    ) -> Option<ListBlock> {
        match node.kind() {
            "list_item" => {
                let mut tree_cursor = node.walk();
                let sub_list = match node.children(&mut tree_cursor).last() {
                    Some(list) if list.kind() == "list" => {
                        let children = list
                            .children(&mut tree_cursor)
                            .flat_map(|it| ListBlock::parse(it, _markdown_tree, rope.clone()))
                            .collect::<Vec<_>>();

                        Some(children)
                    }
                    _ => None,
                };

                let inline_node = node
                    .children(&mut tree_cursor)
                    .find(|it| it.kind() == "paragraph")
                    .and_then(|par| {
                        let mut cursor = par.walk();
                        let x = par.children(&mut cursor).find(|it| it.kind() == "inline");

                        x
                    })?;

                let text_range = inline_node.range();
                let text = rope
                    .byte_slice(text_range.start_byte..text_range.end_byte)
                    .as_str()
                    .unwrap();

                Some(ListBlock {
                    children: sub_list,
                    text_range,
                    text: Arc::from(text),
                    full_range: node.range(),
                })
            }
            _ => None,
        }
    }
}

struct ParagraphBlock {
    range: Range,
    text: Arc<str>,
}

impl Debug for ParagraphBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParagraphBlock")
            .field("text", &self.text)
            .finish()
    }
}

impl ParagraphBlock {
    fn parse(node: tree_sitter::Node, rope: Rope) -> ParagraphBlock {
        let mut cursor = node.walk();
        let mut children = node.children(&mut cursor);
        let inline = children.find(|it| it.kind() == "inline").unwrap();
        let range = inline.range();
        let text = rope
            .byte_slice(range.start_byte..range.end_byte)
            .as_str()
            .unwrap();
        ParagraphBlock {
            range,
            text: Arc::from(text),
        }
    }
}

struct Heading {
    range: Range,
    level: HeadingLevel,
    text: Arc<str>,
}

impl Debug for Heading {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Heading")
            .field("text", &self.text)
            .field("level", &self.level)
            .finish()
    }
}

#[derive(Debug)]
enum HeadingLevel {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}
impl Heading {
    fn parse(it: tree_sitter::Node<'_>, rope: Rope) -> Option<Heading> {
        let mut cursor = it.walk();
        let mut children = it.children(&mut cursor);
        let heading_range = children
            .find(|it| it.kind() == "inline")
            .map(|it| it.range())?;
        let text = rope
            .byte_slice(heading_range.start_byte..heading_range.end_byte)
            .as_str()?;

        let level = it.child(0).and_then(|it| match it.kind() {
            "atx_h1_marker" => Some(HeadingLevel::One),
            "atx_h2_marker" => Some(HeadingLevel::Two),
            "atx_h3_marker" => Some(HeadingLevel::Three),
            "atx_h4_marker" => Some(HeadingLevel::Four),
            "atx_h5_marker" => Some(HeadingLevel::Five),
            "atx_h6_marker" => Some(HeadingLevel::Six),
            _ => None,
        })?;

        Some(Heading {
            range: it.range(),
            level,
            text: Arc::from(text),
        })
    }
}

/// Parse text and return s expression
fn parse(file_text: &str) -> Option<Document> {
    let mut markdown_parser = MarkdownParser::default();
    let tree = markdown_parser.parse(file_text.as_bytes(), None)?;
    let node = tree.walk().node();
    let document = Document::new(node, &tree, file_text);

    document
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let file_text = r#"

# Test

- #Heading
    - Test

Make a *function* for tree-sitter to work with rust well #LATER more text [[Link#HEad]]

"#;

        println!("{:#?}", parse(file_text).unwrap())

        // assert_eq!(file_text, "How will this print?");
    }
}
