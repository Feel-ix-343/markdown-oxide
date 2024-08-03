use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    ops::Not,
    path::Path,
    sync::Arc,
};

use ropey::Rope;
use tree_sitter::Range;
use tree_sitter_md::{MarkdownCursor, MarkdownParser, MarkdownTree};

// struct QueryBlock {
//     sub_blocks: Option<Vec<Arc<QueryBlock>>>,
//     parent: Option<Arc<QueryBlock>>,
//
//     /// Rendered TExt
//     queryable_text: String,
//     collections: Vec<Arc<Collection>>,
// }
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
                        "paragraph" => {
                            match ParagraphBlock::parse(node, markdown_tree, rope.clone()) {
                                Some(par) => vec![Node::Block(BlockContainer::ParagraphBlock(par))],
                                _ => vec![],
                            }
                        }
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
                                .map(|it| BlockContainer::ListBlock(it))
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
    Block(BlockContainer),
    Section(Section),
}

#[derive(Debug)]
enum BlockContainer {
    ListBlock(ListBlock),
    ParagraphBlock(ParagraphBlock),
}

struct ListBlock {
    range: Range,
    content: BlockContent,
    children: Option<Vec<ListBlock>>,
    checkbox: Option<CheckBox>,
}

#[derive(Debug)]
enum CheckBox {
    Checked,
    Unchecked,
}

impl ListBlock {
    fn parse(
        node: tree_sitter::Node,
        markdown_tree: &MarkdownTree,
        rope: Rope,
    ) -> Option<ListBlock> {
        match node.kind() {
            "list_item" => {
                let mut tree_cursor = node.walk();
                let sub_list = match node.children(&mut tree_cursor).last() {
                    Some(list) if list.kind() == "list" => {
                        let children = list
                            .children(&mut tree_cursor)
                            .flat_map(|it| ListBlock::parse(it, markdown_tree, rope.clone()))
                            .collect::<Vec<_>>();

                        Some(children)
                    }
                    _ => None,
                };

                let checkbox = match node.child(1) {
                    Some(node) if node.kind() == "task_list_marker_checked" => {
                        Some(CheckBox::Checked)
                    }
                    Some(node) if node.kind() == "task_list_marker_unchecked" => {
                        Some(CheckBox::Unchecked)
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

                let content = BlockContent::parse(inline_node, rope, markdown_tree)?;

                Some(ListBlock {
                    children: sub_list,
                    content,
                    range: node.range(),
                    checkbox,
                })
            }
            _ => None,
        }
    }
}

impl Debug for ListBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListBlock")
            .field("content", &self.content)
            .field("children", &self.children)
            .field("checkbox", &self.checkbox)
            .finish()
    }
}

struct ParagraphBlock {
    /// Paragraph Range is (row, 0) to (row + 1, 0)
    range: Range,
    content: BlockContent,
}

impl Debug for ParagraphBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParagraphBlock")
            .field("content", &self.content)
            .finish()
    }
}

impl ParagraphBlock {
    fn parse(
        node: tree_sitter::Node,
        markdown_tree: &MarkdownTree,
        rope: Rope,
    ) -> Option<ParagraphBlock> {
        let range = node.range();
        let mut cursor = node.walk();
        let mut children = node.children(&mut cursor);
        let inline = children.find(|it| it.kind() == "inline").unwrap();
        let content = BlockContent::parse(inline, rope.clone(), markdown_tree)?;

        Some(ParagraphBlock { content, range })
    }
}

struct BlockContent {
    text: Arc<str>,
    range: Range,
    tags: Vec<Tag>,
    wiki_links: Vec<WikiLink>,
    md_links: Vec<MarkdownLink>,
}

impl BlockContent {
    fn parse(
        node: tree_sitter::Node,
        rope: Rope,
        markdown_tree: &MarkdownTree,
    ) -> Option<BlockContent> {
        let inline_tree = markdown_tree.inline_tree(&node)?;
        let inline_node = inline_tree.root_node();
        println!("{:?}", inline_node.to_sexp());
        match (inline_node, inline_node.kind()) {
            (node, "inline") => {
                let range = node.range();
                let text = rope.byte_slice(range.start_byte..range.end_byte).as_str()?;

                let mut cursor = node.walk();

                Some(BlockContent {
                    text: Arc::from(text),
                    range,
                    tags: node
                        .children(&mut cursor)
                        .flat_map(|it| Tag::parse(it, rope.clone()))
                        .collect(),
                    md_links: node
                        .children(&mut cursor)
                        .flat_map(|it| MarkdownLink::parse(it, rope.clone()))
                        .collect(),
                    wiki_links: node
                        .children(&mut cursor)
                        .flat_map(|it| WikiLink::parse(it, rope.clone()))
                        .collect(),
                })
            }
            _ => None,
        }
    }
}

impl Debug for BlockContent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockContent")
            .field("text", &self.text)
            .field("tags", &self.tags)
            .field("wiki_links", &self.wiki_links)
            .field("md_links", &self.md_links)
            .finish()
    }
}

struct Tag {
    /// Tag Range, including #
    range: Range,
    /// Tag text no #
    text: Arc<str>,
}

impl Tag {
    fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<Tag> {
        match (inline_child, inline_child.kind()) {
            (node, "tag") => {
                let range = node.range();
                let text_range = range.start_byte + 1..range.end_byte;
                let text = rope.byte_slice(text_range).as_str()?;
                Some(Tag {
                    range,
                    text: Arc::from(text),
                })
            }
            _ => None,
        }
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tag").field("text", &self.text).finish()
    }
}

struct WikiLink {
    range: Range,
    to: Arc<str>,
    display: Option<Arc<str>>,
}

impl WikiLink {
    fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<WikiLink> {
        match (inline_child, inline_child.kind()) {
            (node, "wiki_link") => {
                let range = node.range();
                let to = node.named_child(0).and_then(|it| {
                    let range = it.range();
                    let text = rope.byte_slice(range.start_byte..range.end_byte).as_str()?;
                    Some(Arc::from(text))
                })?;
                let display = node.named_child(1).and_then(|it| {
                    let range = it.range();
                    let text = rope.byte_slice(range.start_byte..range.end_byte).as_str()?;
                    Some(Arc::from(text))
                });
                Some(WikiLink { range, to, display })
            }
            _ => None,
        }
    }
}

impl Debug for WikiLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WikiLink")
            .field("to", &self.to)
            .field("display", &self.display)
            .finish()
    }
}

struct MarkdownLink {
    range: Range,
    to: Arc<str>,
    display: Arc<str>,
}

impl MarkdownLink {
    fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<MarkdownLink> {
        match (inline_child, inline_child.kind()) {
            (node, "inline_link") => {
                let range = node.range();
                let to = node.named_child(1).and_then(|it| {
                    let range = it.range();
                    let text = rope.byte_slice(range.start_byte..range.end_byte).as_str()?;
                    Some(Arc::from(text))
                })?;
                let display = node.named_child(0).and_then(|it| {
                    let range = it.range();
                    let text = rope.byte_slice(range.start_byte..range.end_byte).as_str()?;
                    Some(Arc::from(text))
                })?;
                Some(MarkdownLink { range, to, display })
            }
            _ => None,
        }
    }
}

impl Debug for MarkdownLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownLink")
            .field("to", &self.to)
            .field("display", &self.display)
            .finish()
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

- [ ] Block [[Link|Display]] [NormalLink](Link)
    - #tag Sub Block #tag

Make a *function* for tree-sitter to work with rust well #LATER more text [[Link#HEad]]

- f dj [MarkdownLink](Link)

"#;

        println!("{:#?}", parse(file_text).unwrap())

        // assert_eq!(file_text, "How will this print?");
    }
}
