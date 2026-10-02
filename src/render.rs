use crate::ast::{Document, Node};

pub fn render_document(doc: &Document) -> String {
    render_node(&doc.root, 0)
}

fn render_node(node: &Node, indent: usize) -> String {
    match node {
        Node::Text { content, children: _ } => content.clone(),
        Node::Link { url, label, children: _ } => format!("[{}]({})", label, url),
        Node::Heading { level, children } => {
            let prefix = "#".repeat(*level as usize);
            let content = children.iter().map(|c| render_node(c, indent)).collect::<String>();
            format!("{} {}\n", prefix, content.trim())
        }
        Node::Paragraph { children } => {
            let content = children.iter().map(|c| render_node(c, indent)).collect::<String>();
            format!("{} ", content.trim())
        }
        Node::List { ordered, children } => {
            let mut result = String::new();
            let mut count = 1;
            for child in children {
                match child {
                    Node::ListItem { children: item_children } => {
                        let prefix = if *ordered {
                            format!("{}. ", count)
                        } else {
                            "- ".to_string()
                        };
                        let content = item_children
                            .iter()
                            .map(|c| render_node(c, indent + 1))
                            .collect::<String>();
                        result.push_str(&prefix);
                        result.push_str(&content.trim());
                        result.push('\n');
                        count += 1;
                    }
                    _ => {
                        result.push_str(&render_node(child, indent + 1));
                    }
                }
            }
            result
        }
        Node::ListItem { children } => {
            children.iter().map(|c| render_node(c, indent + 1)).collect()
        }
        Node::CodeBlock { language, code } => {
            format!("```{}\n{}\n```\n", language, code)
        }
        Node::CodeSpan { code } => {
            format!("`{}`", code)
        }
        Node::Emphasis { children } => {
            let content = children.iter().map(|c| render_node(c, indent)).collect::<String>();
            format!("*{}*", content.trim())
        }
        Node::Strong { children } => {
            let content = children.iter().map(|c| render_node(c, indent)).collect::<String>();
            format!("**{}**", content.trim())
        }
        Node::LineBreak => "\n".to_string(),
        Node::HardLineBreak => "  \n".to_string(),
        Node::Blockquote { children } => {
            let mut result = String::new();
            for child in children {
                result.push_str(&format!("> "));
                result.push_str(&render_node(child, indent + 1));
            }
            result
        }
        Node::HtmlBlock { content } => content.clone(),
        Node::ThematicBreak => "---\n".to_string(),
        Node::Table { headers, rows, .. } => {
            let header_line = headers.iter().map(|h| h.clone()).collect::<Vec<_>>().join(" | ");
            let sep_line = headers.iter().map(|_| "---".to_string()).collect::<Vec<_>>().join(" | ");
            let mut result = format!("{} | {} | {}\n", header_line, sep_line);
            for row in rows {
                let row_line = row.iter().map(|c| c.clone()).collect::<Vec<_>>().join(" | ");
                result.push_str(&format!("{} |\n", row_line));
            }
            result
        }
        Node::TableCell { children } => {
            children.iter().map(|c| render_node(c, indent)).collect()
        }
        Node::Unknown { tag, content, attributes: _ } => {
            format!("<!-- {} {} -->", tag, content)
        }
    }
}
