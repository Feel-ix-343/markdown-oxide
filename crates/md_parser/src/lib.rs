use anyhow::anyhow;
use tracing::instrument;
use tree_sitter_md::MarkdownTree;

use std::ops::Not;
use tree_sitter::Range;
use tree_sitter_md::MarkdownParser;

use std::fmt::Formatter;

use std::fmt::Debug;
use ropey::Rope;

pub struct Document {
    pub sections: Vec<Section>,
    rope: Rope,
}

pub struct Section {
    doc_rope: Rope,

    /// what is this? this is the full range of the section, including the heading and all contained content.
    range: Range,
    pub heading: Option<Heading>,
    pub level: usize,
    pub nodes: Vec<Node>,
}

impl Debug for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Section")
            .field("range", &self.range)
            .field("heading", &self.heading)
            .field("level", &self.level)
            .field("nodes", &self.nodes)
            .finish()
    }
}

#[derive(Debug)]
pub enum Node {
    Block(DocBlock),
    Section(Section),
}

#[derive(Debug)]
pub enum DocBlock {
    ListBlock(ListBlock),
    ParagraphBlock(ParagraphBlock),
}

pub struct ListBlock {
    /// what is this?
    /// this is the exact range of the list block and does not contain the ranges of the children blocks.
    /// while hte BlockContent range includes only the text content of the block, this will include other markers
    /// like the checkbox and the `- `
    pub range: Range,
    /// what is this?
    /// this is the exact content of the list block and does not contain the content of the children blocks.
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
    doc_rope: Rope,
    /// This is the exact range of only the block's content, excluding any (list) block markers like `-` ...
    pub range: Range,
    pub tags: Vec<Tag>,
    pub wiki_links: Vec<WikiLink>,
    pub md_links: Vec<MarkdownLink>,
}

pub struct Tag {
    /// Tag Range, including #
    doc_rope: Rope,
    pub range: Range,
}

pub struct WikiLink {
    doc_rope: Rope,
    /// The full range of the wiki link, including the [[]] markers
    /// For example, in "[[Page]]", includes "[[Page]]"
    range: Range,
    /// The range of just the target page name
    /// For example, in "[[Page]]", includes just "Page"
    to_range: Range,
    /// The range of the optional display text after the |
    /// For example, in "[[Page|Display]]", includes just "Display"
    display_range: Option<Range>,
}

pub struct MarkdownLink {
    doc_rope: Rope,
    pub range: Range,
    pub to_range: Range,
    pub display_range: Range,
}

pub struct Heading {
    /// The underlying rope data structure containing the document text
    doc_rope: Rope,
    /// The range of just the heading text content, excluding the '#' markers
    /// For example, in "## Heading", this range only includes "Heading"
    range: Range,
    /// The range of the complete heading including the '#' markers and content
    /// For example, in "## Heading", this range includes "## Heading"
    full_range: Range,
    /// The heading level (h1-h6) determined by the number of '#' markers
    pub level: HeadingLevel,
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
    #[instrument(level = "debug")]
    pub fn new(text: &str) -> anyhow::Result<Document> {
        let mut markdown_parser = MarkdownParser::default();
        let markdown_tree = markdown_parser
            .parse(text.as_bytes(), None)
            .ok_or(anyhow!("Treesitter failed to parse"))?;
        let node = markdown_tree.walk().node();

        let rope = Rope::from_str(text);
        match node.kind() {
            "document" => {
                let mut cursor = node.walk();
                let children = node.children(&mut cursor);
                let sections = children
                    .flat_map(|it| Section::parse_section(it, &markdown_tree, 0, rope.clone()))
                    .collect();
                Ok(Document { sections, rope })
            }
            k => Err(anyhow!("Failed to parse document at top level: {k:?}")),
        }
    }
}

/// Behavior
impl Document {
    pub fn content(&self) -> String {
        self.rope.to_string()
    }
    pub fn all_doc_blocks(&self) -> Box<dyn Iterator<Item = BorrowedDocBlock<'_>> + '_> {
        Box::new(self.sections.iter().flat_map(|it| it.all_blocks()))
    }

    pub fn sections(&self) -> Box<dyn Iterator<Item = &Section> + '_> {
        Box::new(self.sections.iter().flat_map(|section| {
            std::iter::once(section).chain(
                section
                    .nodes
                    .iter()
                    .filter_map(|node| {
                        if let Node::Section(subsection) = node {
                            Some(subsection.sections())
                        } else {
                            None
                        }
                    })
                    .flatten(),
            )
        }))
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
                let section_range = node.range();
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
                                        ListBlock::parse(child, markdown_tree, rope.clone())
                                    }
                                    _ => None,
                                })
                                .map(|it| DocBlock::ListBlock(it))
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
                        doc_rope: rope,
                        heading,
                        level,
                        nodes,
                        range: section_range,
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
    /// Recursive iterator through all blocks in the section
    pub fn all_blocks(&self) -> Box<dyn Iterator<Item = BorrowedDocBlock> + '_> {
        Box::new(self.nodes.iter().flat_map(|it| match it {
            Node::Block(DocBlock::ListBlock(list_block)) => {
                Box::new(list_block.blocks().map(BorrowedDocBlock::ListBlock))
                    as Box<dyn Iterator<Item = BorrowedDocBlock>>
            }
            Node::Block(DocBlock::ParagraphBlock(paragraph_block)) => Box::new(std::iter::once(
                BorrowedDocBlock::ParagraphBlock(paragraph_block),
            )),
            Node::Section(section) => section.all_blocks(),
        }))
    }

    /// Iterator through just the top level blocks in a section
    pub fn top_level_blocks(&self) -> Box<dyn Iterator<Item = &DocBlock> + '_> {
        Box::new(self.nodes.iter().filter_map(|node| match node {
            Node::Block(block) => Some(block),
            Node::Section(_) => None,
        }))
    }

    pub fn sections(&self) -> Box<dyn Iterator<Item = &Section> + '_> {
        Box::new(
            std::iter::once(self).chain(
                self.nodes
                    .iter()
                    .filter_map(|node| {
                        if let Node::Section(subsection) = node {
                            Some(subsection.sections())
                        } else {
                            None
                        }
                    })
                    .flatten(),
            ),
        )
    }

    pub fn content(&self) -> String {
        self.doc_rope
            .byte_slice(self.range.start_byte..self.range.end_byte)
            .to_string()
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
            .field("range", &self.range)
            .finish()
    }
}

impl Debug for ParagraphBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParagraphBlock")
            .field("content", &self.content)
            .field("range", &self.range)
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
                let mut cursor = node.walk();

                Some(BlockContent {
                    doc_rope: rope.clone(),
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

    pub fn refs(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(
            self.tags
                .iter()
                .map(|tag| tag.text())
                .chain(self.md_links.iter().map(|link| link.to()))
                .chain(self.wiki_links.iter().map(|link| link.to()))
        )
    }
}

impl Debug for BlockContent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockContent")
            .field("content", &self.content())
            .field("tags", &self.tags)
            .field("wiki_links", &self.wiki_links)
            .field("md_links", &self.md_links)
            .field("range", &self.range)
            .finish()
    }
}

impl Tag {
    pub fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<Tag> {
        match (inline_child, inline_child.kind()) {
            (node, "tag") => {
                Some(Tag {
                    doc_rope: rope,
                    range: node.range(),
                })
            }
            _ => None,
        }
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tag").field("text", &self.text()).finish()
    }
}

impl WikiLink {
    pub fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<WikiLink> {
        match (inline_child, inline_child.kind()) {
            (node, "wiki_link") => {
                let to_range = node.named_child(0)?.range();
                let display_range = node.named_child(1).map(|it| it.range());
                
                Some(WikiLink {
                    doc_rope: rope,
                    range: node.range(),
                    to_range,
                    display_range,
                })
            }
            _ => None,
        }
    }
}

impl Debug for WikiLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WikiLink")
            .field("to", &self.to())
            .field("display", &self.display())
            .finish()
    }
}

impl MarkdownLink {
    pub fn parse(inline_child: tree_sitter::Node, rope: Rope) -> Option<MarkdownLink> {
        match (inline_child, inline_child.kind()) {
            (node, "inline_link") => {
                let to_range = node.named_child(1)?.range();
                let display_range = node.named_child(0)?.range();
                
                Some(MarkdownLink {
                    doc_rope: rope,
                    range: node.range(),
                    to_range,
                    display_range,
                })
            }
            _ => None,
        }
    }
}

impl Debug for MarkdownLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownLink")
            .field("to", &self.to())
            .field("display", &self.display())
            .finish()
    }
}

impl Debug for Heading {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Heading")
            .field("text", &self.heading_text())
            .field("level", &self.level)
            .field("range", &self.range)
            .finish()
    }
}

impl Heading {
    pub fn parse(it: tree_sitter::Node<'_>, rope: Rope) -> Option<Heading> {
        let mut cursor = it.walk();
        let mut children = it.children(&mut cursor);
        // The heading_range only includes the content after the '#' markers
        // For example, in "## Heading", it only includes "Heading"
        let heading_range = children
            .find(|it| it.kind() == "inline")
            .map(|it| it.range())?;

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
            doc_rope: rope,
            range: heading_range,
            full_range: it.range(),
            level,
        })
    }
}


impl BlockContent {
    pub fn content(&self) -> String {
        self.doc_rope
            .byte_slice(self.range.start_byte..self.range.end_byte)
            .to_string()
    }
}

impl Tag {
    pub fn text(&self) -> String {
        // Skip the # character by adding 1 to start_byte
        self.doc_rope
            .byte_slice(self.range.start_byte + 1..self.range.end_byte)
            .to_string()
    }
}

impl WikiLink {
    pub fn to(&self) -> String {
        self.doc_rope
            .byte_slice(self.to_range.start_byte..self.to_range.end_byte)
            .to_string()
    }

    pub fn display(&self) -> Option<String> {
        self.display_range.map(|range| {
            self.doc_rope
                .byte_slice(range.start_byte..range.end_byte)
                .to_string()
        })
    }
}

impl MarkdownLink {
    pub fn to(&self) -> String {
        self.doc_rope
            .byte_slice(self.to_range.start_byte..self.to_range.end_byte)
            .to_string()
    }

    pub fn display(&self) -> String {
        self.doc_rope
            .byte_slice(self.display_range.start_byte..self.display_range.end_byte)
            .to_string()
    }
}

impl Heading {
    pub fn heading_text(&self) -> String {
        self.doc_rope
            .byte_slice(self.range.start_byte..self.range.end_byte)
            .to_string()
    }

    pub fn full_heading_content(&self) -> String {
        self.doc_rope
            .byte_slice(self.full_range.start_byte..self.full_range.end_byte)
            .to_string()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let file_text = r#"# Baker v Carr
[[Members of Congress#Redistricting]]

One person one vote: [[Baker v Carr#^baa84a]]
## Facts
- Tennessee citizens (Baker was alphabetically first in this list of citizens) thought that Tennessee's districting, which was set up based on the distribution of population in 1901, was unfair to current population shifts; they were based on geography instead of population and were thought to be unfair bu the legislatures did not want to lose their seats though fairer redistricting
  - Rural parts of states would have the same representation as a largely populated city
  - Rural parts of states would take over the legislatures while there was much more population in the cities
    - The population in 1901 must have been much more spread out, then became compressed as time passed
    - **As time passed, the legislature who had been elected there at the time did not want to lose their seats by redistricting, so they did nto do it**
  - **Districts did not have equal populations, and therefore, the votes of each person were unequal**
- Prior to this trial, the supreme court had refused to intervine in apportionment cases
  - I am guessing that these cases would be for a similar topic
  - Supreme court thought that these cases were not coverable under the constitution; it did not have anything to say about them
  - Reversed this decision for this case (Q1: is this something under the power of the constitution)
## Question
- Did the supreme court have jurisdiction over questions of legislative apportiontmentp
- Should all votes should be represented equally
  - Gerrymandering changes the ratios of voters of parties to the actual districts made; dilutes
## Majority Opinion and Reasoning
- Favor of Baker: The supreme court ruile dthat they coukld review state redistricting issues and that all districts should be proportionately represented
### Reasoning
- Cited the **equal protection clause** (14th ammendment) gave citizens an equal vote that should not be based on geography
  - Answering Q1: [**IMPACT**] this means that redistricting cases are **justiciable**; other cases were held
  - Answering Q2: The districts can't undermine equality; ruled in favor of baker; in future cases, "one person, one vote" was required by the constitution ^baa84a"#;

        println!("{:#?}", Document::new(file_text).unwrap())

        // assert_eq!(file_text, "How will this print?");
    }
}
