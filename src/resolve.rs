use crate::ast::{Document, Node};
use crate::parse;
use std::collections::HashSet;

/// Error type for unresolved references
#[derive(Debug, Clone)]
pub struct ReferenceError {
    pub text: String,
    pub line: usize,
    pub col: usize,
}

/// Resolve wiki-style references `[[target]]` in the document.
///
/// A valid reference must have both `[[` and `]]` within the same text node
/// (or same inline segment). Isolated `[` or `]` characters that do not form
/// a complete `[[...]]` pair are ignored.
pub fn resolve_document(doc: &Document) -> Result<Document, Vec<ReferenceError>> {
    let mut errors = Vec::new();
    let resolved_tree = resolve_node(&doc.root, &mut errors);
    if errors.is_empty() {
        Ok(Document { root: resolved_tree })
    } else {
        Err(errors)
    }
}

fn resolve_node(node: &Node, errors: &mut Vec<ReferenceError>) -> Node {
    let children: Vec<Node> = node
        .children
        .iter()
        .map(|child| resolve_node(child, errors))
        .collect();

    match node {
        Node::Text { content, .. } => {
            // Only treat content as a reference candidate if it contains BOTH
            // `[[` and `]]` — i.e. they appear in the same text node.
            let has_open = content.contains("[[");
            let has_close = content.contains("]]");

            if has_open && has_close {
                // Try to parse as a wiki reference
                if let Some(parsed) = parse_reference(content) {
                    return parsed;
                }
            }

            // Not a reference (or could not be parsed) — keep as plain text
            Node::Text {
                content: content.clone(),
                children,
            }
        }
        Node::Link { url, label, .. } => Node::Link {
            url: url.clone(),
            label,
            children,
        },
        Node::Heading { level, children } => Node::Heading {
            level: *level,
            children,
        },
        Node::Paragraph { children } => Node::Paragraph { children },
        Node::List { ordered, children, .. } => Node::List {
            ordered: *ordered,
            children,
        },
        Node::ListItem { children, .. } => Node::ListItem { children },
        Node::CodeBlock { language, code, .. } => Node::CodeBlock {
            language: language.clone(),
            code: code.clone(),
        },
        Node::CodeSpan { code, .. } => Node::CodeSpan {
            code: code.clone(),
        },
        Node::Emphasis { children } => Node::Emphasis { children },
        Node::Strong { children } => Node::Strong { children },
        Node::LineBreak => Node::LineBreak,
        Node::HardLineBreak => Node::HardLineBreak,
        Node::Blockquote { children } => Node::Blockquote { children },
        Node::HtmlBlock { content, .. } => Node::HtmlBlock {
            content: content.clone(),
        },
        Node::ThematicBreak => Node::ThematicBreak,
        Node::Table { headers, rows, alignments } => Node::Table {
            headers: headers.clone(),
            rows: rows.clone(),
            alignments: alignments.clone(),
        },
        Node::TableCell { children, .. } => Node::TableCell { children },
        Node::Unknown { tag, content, attributes } => Node::Unknown {
            tag: tag.clone(),
            content: content.clone(),
            attributes: attributes.clone(),
        },
    }
}

/// Try to parse a wiki-style reference from a text string.
/// Returns `Some(Node)` if the entire content is a single reference,
/// `None` otherwise (content is returned as plain text).
fn parse_reference(content: &str) -> Option<Node> {
    let s = content.trim();
    if !s.starts_with("[[") || !s.ends_with("]]") {
        return None;
    }
    // Remove the outer `[[` and `]]`
    let inner = &s[2..s.len() - 2];
    if inner.is_empty() {
        return None;
    }

    // Split on `|` for label
    let (target, label) = if let Some(pos) = inner.find('|') {
        let t = inner[..pos].trim().to_string();
        let l = inner[pos + 1..].trim().to_string();
        (t, l)
    } else {
        (inner.trim().to_string(), inner.trim().to_string())
    };

    if target.is_empty() {
        return None;
    }

    Some(Node::Link {
        url: target,
        label,
        children: vec![],
    })
}

/// Collect all unresolved reference errors from a document.
pub fn collect_reference_errors(doc: &Document) -> Vec<ReferenceError> {
    let mut errors = Vec::new();
    resolve_document(doc).unwrap_or_else(|e| errors.extend(e));
    errors
}
