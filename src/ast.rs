use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Node {
    Text {
        content: String,
        children: Vec<Node>,
    },
    Link {
        url: String,
        label: String,
        children: Vec<Node>,
    },
    Heading {
        level: u8,
        children: Vec<Node>,
    },
    Paragraph {
        children: Vec<Node>,
    },
    List {
        ordered: bool,
        children: Vec<Node>,
    },
    ListItem {
        children: Vec<Node>,
    },
    CodeBlock {
        language: String,
        code: String,
    },
    CodeSpan {
        code: String,
    },
    Emphasis {
        children: Vec<Node>,
    },
    Strong {
        children: Vec<Node>,
    },
    LineBreak,
    HardLineBreak,
    Blockquote {
        children: Vec<Node>,
    },
    HtmlBlock {
        content: String,
    },
    ThematicBreak,
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        alignments: Vec<pulldown_cmark::Alignment>,
    },
    TableCell {
        children: Vec<Node>,
    },
    Unknown {
        tag: String,
        content: String,
        attributes: Vec<(String, String)>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub root: Node,
}
