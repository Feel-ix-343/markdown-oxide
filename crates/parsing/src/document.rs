use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use tree_sitter_md::MarkdownParser;

use std::ops::Not;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use tree_sitter::Range;

use tree_sitter_md::MarkdownTree;

use std::fmt::Formatter;

use std::fmt::Debug;

use ropey::Rope;

pub(crate) struct Document {
    pub(crate) sections: Vec<DocSection>,
    pub(crate) rope: Rope,
}

#[derive(Debug)]
pub(crate) struct DocSection {
    pub(crate) heading: Option<Heading>,
    pub(crate) level: usize,
    pub(crate) nodes: Vec<Node>,
}

#[derive(Debug)]
pub(crate) enum Node {
    Block(DocBlock),
    Section(DocSection),
}

#[derive(Debug, Clone)]
pub(crate) enum DocBlock {
    ListBlock(DocListBlock),
    ParagraphBlock(DocParagraphBlock),
}

#[derive(Clone)]
pub(crate) struct DocListBlock {
    pub(crate) range: Range,
    pub(crate) content: BlockContent,
    pub(crate) children: Option<Vec<DocListBlock>>,
    pub(crate) checkbox: Option<CheckBox>,
}

#[derive(Debug, Clone)]
pub(crate) enum CheckBox {
    Checked,
    Unchecked,
}

#[derive(Clone)]
pub(crate) struct DocParagraphBlock {
    /// Paragraph Range is (row, 0) to (row + 1, 0)
    pub(crate) range: Range,
    pub(crate) content: BlockContent,
}

#[derive(Clone)]
pub(crate) struct BlockContent {
    pub(crate) text: Arc<str>,
    pub(crate) range: Range,
    pub(crate) tags: Vec<Tag>,
    pub(crate) wiki_links: Vec<WikiLink>,
    pub(crate) md_links: Vec<MarkdownLink>,
    pub(crate) index: Option<Arc<str>>,
}

#[derive(Clone)]
pub(crate) struct Tag {
    /// Tag Range, including #
    pub(crate) range: Range,
    /// Tag text no #
    pub(crate) text: Arc<str>,
}

#[derive(Clone)]
pub(crate) struct WikiLink {
    pub(crate) range: Range,
    pub(crate) to: Arc<str>,
    pub(crate) display: Option<Arc<str>>,
}

#[derive(Clone)]
pub(crate) struct MarkdownLink {
    pub(crate) range: Range,
    pub(crate) to: Arc<str>,
    pub(crate) display: Arc<str>,
}

pub(crate) struct Heading {
    pub(crate) range: Range,
    pub(crate) level: HeadingLevel,
    pub(crate) text: Arc<str>,
}

#[derive(Debug)]
pub(crate) enum HeadingLevel {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl Debug for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("sections", &self.sections)
            .finish()
    }
}

/// Document behavior
impl Document {
    pub(crate) fn top_level_doc_blocks(&self) -> impl Iterator<Item = &DocBlock> + '_ {
        self.sections
            .iter()
            .map(|it| it.top_level_blocks())
            .flatten()
    }

    pub(crate) fn all_blocks(&self) -> Box<dyn Iterator<Item = DocBlock> + '_> {
        Box::new(
            self.sections
                .iter()
                .map(|section| section.all_blocks())
                .flatten(),
        )
    }
}

/// DocBlock behavior
impl DocBlock {
    fn content(&self) -> &BlockContent {
        match self {
            Self::ListBlock(b) => &b.content,
            Self::ParagraphBlock(b) => &b.content,
        }
    }

    pub(crate) fn doc_index(&self) -> Option<Arc<str>> {
        self.content().index.clone()
    }
}

impl From<DocListBlock> for DocBlock {
    fn from(b: DocListBlock) -> Self {
        Self::ListBlock(b)
    }
}

impl From<DocParagraphBlock> for DocBlock {
    fn from(b: DocParagraphBlock) -> Self {
        Self::ParagraphBlock(b)
    }
}

/// Document construction
impl Document {
    pub(crate) fn new(text: &str) -> Option<Document> {
        let mut markdown_parser = MarkdownParser::default();
        let markdown_tree = markdown_parser.parse(text.as_bytes(), None)?;
        let node = markdown_tree.walk().node();

        let rope = Rope::from_str(text);
        match node.kind() {
            "document" => {
                let now = std::time::Instant::now();
                let mut cursor = node.walk();
                let children = node.children(&mut cursor);
                let sections = children
                    .flat_map(|it| DocSection::parse_section(it, &markdown_tree, 0, rope.clone()))
                    .collect();
                let elapsed = now.elapsed();
                Some(Document { sections, rope })
            }
            _ => None,
        }
    }
}

impl DocSection {
    fn parse_section(
        node: tree_sitter::Node,
        markdown_tree: &MarkdownTree,
        level: usize,
        rope: Rope,
    ) -> Option<DocSection> {
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
                            match DocParagraphBlock::parse(node, markdown_tree, rope.clone()) {
                                Some(par) => {
                                    vec![Node::Block(DocBlock::ParagraphBlock(par))]
                                }
                                _ => vec![],
                            }
                        }
                        "list" => {
                            let mut cursor = node.walk();
                            let children = node.children(&mut cursor);
                            children
                                .flat_map(|child| match child.kind() {
                                    "list_item" => {
                                        DocListBlock::parse(child, markdown_tree, rope.clone())
                                    }
                                    _ => None,
                                })
                                .map(|it| DocBlock::ListBlock(it))
                                .map(|it| Node::Block(it))
                                .collect::<Vec<_>>()
                        }
                        "section" => {
                            DocSection::parse_section(node, markdown_tree, level + 1, rope.clone())
                                .map(|it| vec![Node::Section(it)])
                                .unwrap_or(Vec::new())
                        } // need my monad trans
                        _ => vec![],
                    })
                    .collect();

                if nodes.is_empty().not() || heading.is_some() {
                    Some(DocSection {
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

// Behavior
impl DocSection {
    fn top_level_blocks(&self) -> Box<dyn Iterator<Item = &DocBlock> + '_> {
        Box::new(
            self.nodes
                .iter()
                .map(|it| match it {
                    Node::Block(block) => Box::new(std::iter::once(block)),
                    Node::Section(section) => section.top_level_blocks(),
                })
                .flatten(),
        )
    }

    fn all_blocks(&self) -> Box<dyn Iterator<Item = DocBlock> + '_> {
        Box::new(
            self.nodes
                .iter()
                .map(|it| match it {
                    Node::Block(block) => match block {
                        p @ DocBlock::ParagraphBlock(b) => Box::new(std::iter::once(p.clone()))
                            as Box<dyn Iterator<Item = DocBlock>>,
                        DocBlock::ListBlock(block) => Box::new(block.list_blocks().map(|it| {
                            let cloned = it.to_owned();
                            DocBlock::from(cloned)
                        })),
                    },
                    Node::Section(section) => section.all_blocks(),
                })
                .flatten(),
        )
    }
}

/// Behavior
impl DocListBlock {
    pub(crate) fn list_blocks(&self) -> Box<dyn Iterator<Item = &DocListBlock> + '_> {
        Box::new(
            std::iter::once(self).chain(
                self.children
                    .iter()
                    .flatten()
                    .map(|child| child.list_blocks())
                    .flatten(),
            ),
        )
    }
}

impl DocListBlock {
    fn parse(
        node: tree_sitter::Node,
        markdown_tree: &MarkdownTree,
        rope: Rope,
    ) -> Option<DocListBlock> {
        match node.kind() {
            "list_item" => {
                let mut tree_cursor = node.walk();
                let sub_list = match node.children(&mut tree_cursor).last() {
                    Some(list) if list.kind() == "list" => {
                        let children = list
                            .children(&mut tree_cursor)
                            .flat_map(|it| DocListBlock::parse(it, markdown_tree, rope.clone()))
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

                Some(DocListBlock {
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

impl Debug for DocListBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListBlock")
            .field("content", &self.content)
            .field("children", &self.children)
            .field("checkbox", &self.checkbox)
            .finish()
    }
}

impl Debug for DocParagraphBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParagraphBlock")
            .field("content", &self.content)
            .finish()
    }
}

impl DocParagraphBlock {
    fn parse(
        node: tree_sitter::Node,
        markdown_tree: &MarkdownTree,
        rope: Rope,
    ) -> Option<DocParagraphBlock> {
        let range = node.range();
        let mut cursor = node.walk();
        let mut children = node.children(&mut cursor);
        let inline = children.find(|it| it.kind() == "inline").unwrap();
        let content = BlockContent::parse(inline, rope.clone(), markdown_tree)?;

        Some(DocParagraphBlock { content, range })
    }
}

/// Behavior
impl BlockContent {
    pub(crate) fn link_refs(&self) -> impl Iterator<Item = &str> + '_ {
        self.md_links
            .iter()
            .map(|it| it.to.as_ref())
            .chain(self.wiki_links.iter().map(|it| it.to.as_ref()))
    }
}

impl BlockContent {
    fn parse(
        node: tree_sitter::Node,
        rope: Rope,
        markdown_tree: &MarkdownTree,
    ) -> Option<BlockContent> {
        let inline_tree = markdown_tree.inline_tree(&node)?;
        let inline_node = inline_tree.root_node();
        match (inline_node, inline_node.kind()) {
            (node, "inline") => {
                let range = node.range();
                let text = rope.byte_slice(range.start_byte..range.end_byte).as_str()?;

                static INDEX_RE: Lazy<Regex> =
                    Lazy::new(|| Regex::new(r" \^(?<index>[\w-]+)$").unwrap());
                let index = INDEX_RE
                    .captures(text)
                    .and_then(|capture| capture.name("index"))
                    .map(|it| Arc::from(it.as_str()));

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
                    index,
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
            .field("index", &self.index)
            .finish()
    }
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

impl Debug for Heading {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Heading")
            .field("text", &self.text)
            .field("level", &self.level)
            .finish()
    }
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    //     #[test]
    //     fn test_parse() {
    //         let file_text = r#"
    //
    // # Test
    //
    // - [ ] Block [[Link|Display]] [NormalLink](Link)
    //     - #tag Sub Block #tag
    //
    // Make a *function* for tree-sitter to work with rust well #LATER more text [[Link#HEad]]
    //
    // - f dj [MarkdownLink](Link)
    //
    // "#;
    //
    //         println!("{:#?}", parse(file_text).unwrap())
    //
    //         // assert_eq!(file_text, "How will this print?");
    //     }
}
