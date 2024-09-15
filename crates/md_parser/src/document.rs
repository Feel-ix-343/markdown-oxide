use rayon::prelude::*;
use tree_sitter_md::MarkdownParser;

use std::ops::Not;
use std::sync::Arc;
use std::time::Duration;

use tree_sitter::Range;

use tree_sitter_md::MarkdownTree;

use std::fmt::Formatter;

use std::fmt::Debug;

use ropey::Rope;

pub struct Document {
    pub sections: Vec<Section>,
    pub rope: Rope,
}

#[derive(Debug)]
pub struct Section {
    pub heading: Option<Heading>,
    pub level: usize,
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub enum Node {
    Block(BlockContainer),
    Section(Section),
}

#[derive(Debug)]
pub enum BlockContainer {
    ListBlock(ListBlock),
    ParagraphBlock(ParagraphBlock),
}

pub struct ListBlock {
    pub range: Range,
    pub content: BlockContent,
    pub children: Option<Vec<ListBlock>>,
    pub checkbox: Option<CheckBox>,
}

#[derive(Debug)]
pub enum CheckBox {
    Checked,
    Unchecked,
}

pub struct ParagraphBlock {
    /// Paragraph Range is (row, 0) to (row + 1, 0)
    pub range: Range,
    pub content: BlockContent,
}

pub struct BlockContent {
    pub text: Arc<str>,
    pub range: Range,
    pub tags: Vec<Tag>,
    pub wiki_links: Vec<WikiLink>,
    pub md_links: Vec<MarkdownLink>,
}

pub struct Tag {
    /// Tag Range, including #
    pub range: Range,
    /// Tag text no #
    pub text: Arc<str>,
}

pub struct WikiLink {
    pub range: Range,
    pub to: Arc<str>,
    pub display: Option<Arc<str>>,
}

pub struct MarkdownLink {
    pub range: Range,
    pub to: Arc<str>,
    pub display: Arc<str>,
}

pub struct Heading {
    pub range: Range,
    pub level: HeadingLevel,
    pub text: Arc<str>,
}

#[derive(Debug)]
pub enum HeadingLevel {
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

impl Document {
    pub fn new(text: &str) -> Option<Document> {
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
                    .flat_map(|it| Section::parse_section(it, &markdown_tree, 0, rope.clone()))
                    .collect();
                let elapsed = now.elapsed();
                Some(Document { sections, rope })
            }
            _ => None,
        }
    }
}

/// Behavior
impl Document {
    pub fn all_doc_blocks(&self) -> Box<dyn Iterator<Item = BorrowedDocBlock<'_>> + '_> {
        Box::new(self.sections.iter().flat_map(|it| it.blocks()))
    }
}

impl Section {
    pub fn parse_section(
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
                                Some(par) => {
                                    vec![Node::Block(BlockContainer::ParagraphBlock(par))]
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

pub enum BorrowedDocBlock<'a> {
    ListBlock(&'a ListBlock),
    ParagraphBlock(&'a ParagraphBlock),
}

impl BorrowedDocBlock<'_> {
    pub fn content(&self) -> &BlockContent {
        match self {
            BorrowedDocBlock::ListBlock(block) => &block.content,
            BorrowedDocBlock::ParagraphBlock(block) => &block.content,
        }
    }

    pub fn line_number(&self) -> usize {
        self.content().range.start_point.row + 1
    }
}

impl Section {
    fn blocks(&self) -> Box<dyn Iterator<Item = BorrowedDocBlock> + '_> {
        Box::new(self.nodes.iter().flat_map(|it| match it {
            Node::Block(BlockContainer::ListBlock(list_block)) => {
                Box::new(list_block.blocks().map(BorrowedDocBlock::ListBlock))
                    as Box<dyn Iterator<Item = BorrowedDocBlock>>
            }
            Node::Block(BlockContainer::ParagraphBlock(paragraph_block)) => Box::new(
                std::iter::once(BorrowedDocBlock::ParagraphBlock(paragraph_block)),
            ),
            Node::Section(section) => section.blocks(),
        }))
    }
}

impl ListBlock {
    pub fn parse(
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

impl ListBlock {
    fn blocks(&self) -> Box<dyn Iterator<Item = &ListBlock> + '_> {
        let children_iter = self
            .children
            .iter()
            .flatten()
            .flat_map(|child| child.blocks());
        Box::new(std::iter::once(self).chain(children_iter))
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

impl Debug for ParagraphBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParagraphBlock")
            .field("content", &self.content)
            .finish()
    }
}

impl ParagraphBlock {
    pub fn parse(
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

impl BlockContent {
    pub fn parse(
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

    pub fn refs(&self) -> Box<dyn Iterator<Item = Arc<str>> + '_> {
        Box::new(
            self.tags
                .iter()
                .map(|tag| tag.text.clone())
                .chain(self.md_links.iter().map(|link| link.to.clone()))
                .chain(self.wiki_links.iter().map(|link| link.to.clone())),
        )
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

impl Tag {
    pub fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<Tag> {
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
    pub fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<WikiLink> {
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
    pub fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<MarkdownLink> {
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
    pub fn parse(it: tree_sitter::Node<'_>, rope: Rope) -> Option<Heading> {
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
