```rust
use crate::ast::{Document, Node};
use pulldown_cmark::{Parser, Event, Tag, TagEnd, Options};

/// Parse a markdown string into an AST Document.
pub fn parse_document(markdown: &str) -> Result<Document, String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);

    let root = parse_events(parser)?;
    Ok(Document { root })
}

fn parse_events(events: impl Iterator<Item = Event<'_>>) -> Result<Node, String> {
    let mut children = Vec::new();
    let mut current_tag: Option<Tag<'_>> = None;
    let mut text_buf = String::new();
    let mut code_buf = String::new();
    let mut in_code = false;
    let mut code_lang = String::new();

    for event in events {
        match event {
            Event::Start(tag) => {
                if in_code {
                    // We were inside a code span/block — nested starts shouldn't happen
                    // in well-formed markdown, but we handle gracefully
                }
                current_tag = Some(tag);
            }
            Event::End(tag) => {
                if let Some(ref mut open) = current_tag {
                    let open_kind = std::mem::replace(open, Tag::Paragraph(vec![]));
                    match (&open_kind, tag.kind()) {
                        (Tag::Paragraph(_), TagEnd::Paragraph) => {
                            let node = if text_buf.is_empty() {
                                Node::Paragraph { children: vec![] }
                            } else {
                                let content = std::mem::take(&mut text_buf);
                                Node::Text {
                                    content,
                                    children: vec![],
                                }
                            };
                            children.push(node);
                        }
                        (Tag::Heading(level, _), TagEnd::Heading(l)) => {
                            let h_level = **level as u8;
                            let heading_children = std::mem::take(&mut children);
                            children.push(Node::Heading {
                                level: h_level,
                                children: heading_children,
                            });
                        }
                        (Tag::Emphasis, TagEnd::Emphasis) => {
                            let emp_children = std::mem::take(&mut children);
                            children.push(Node::Emphasis { children: emp_children });
                        }
                        (Tag::Strong, TagEnd::Strong) => {
                            let strong_children = std::mem::take(&mut children);
                            children.push(Node::Strong { children: strong_children });
                        }
                        (Tag::Link(_, url, _), TagEnd::Link) => {
                            let link_children = std::mem::take(&mut children);
                            children.push(Node::Link {
                                url: url.to_string(),
                                label: String::new(),
                                children: link_children,
                            });
                        }
                        (Tag::Image(_, url, _), TagEnd::Image) => {
                            let img_children = std::mem::take(&mut children);
                            children.push(Node::Link {
                                url: url.to_string(),
                                label: String::new(),
                                children: img_children,
                            });
                        }
                        (Tag::CodeBlock(kind), TagEnd::CodeBlock) => {
                            in_code = false;
                            let lang = match kind {
                                pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                                pulldown_cmark::CodeBlockKind::Indented => String::new(),
                            };
                            let code = std::mem::take(&mut code_buf);
                            children.push(Node::CodeBlock {
                                language: lang,
                                code,
                            });
                        }
                        (Tag::Item, TagEnd::Item) => {
                            let item_children = std::mem::take(&mut children);
                            children.push(Node::ListItem { children: item_children });
                        }
                        (Tag::List(ordered), TagEnd::List) => {
                            let list_children = std::mem::take(&mut children);
                            children.push(Node::List {
                                ordered: ordered.is_some(),
                                children: list_children,
                            });
                        }
                        (Tag::BlockQuote, TagEnd::BlockQuote) => {
                            let bq_children = std::mem::take(&mut children);
                            children.push(Node::Blockquote { children: bq_children });
                        }
                        (Tag::Table(col_aligns), TagEnd::Table) => {
                            let table_children = std::mem::take(&mut children);
                            let alignments: Vec<pulldown_cmark::Alignment> =
                                col_aligns.iter().map(|a| *a).collect();
                            // Parse table rows
                            let (headers, rows) = parse_table_children(table_children, &alignments);
                            children.push(Node::Table {
                                headers,
                                rows,
                                alignments,
                            });
                        }
                        (Tag::TableHead, TagEnd::TableHead) => {
                            let head_children = std::mem::take(&mut children);
                            // This is handled inside Table
                        }
                        (Tag::TableRow, TagEnd::TableRow) => {
                            let row_children = std::mem::take(&mut children);
                            // This is handled inside Table
                        }
                        (Tag::TableCell, TagEnd::TableCell) => {
                            let cell_children = std::mem::take(&mut children);
                            children.push(Node::TableCell { children: cell_children });
                        }
                        _ => {
                            // For unmatched tags, try to keep children
                            children.push(Node::Unknown {
                                tag: format!("{:?}", open_kind.kind()),
                                content: String::new(),
                                attributes: vec![],
                            });
                        }
                    }
                    current_tag = None;
                }
            }
            Event::Text(text) => {
                if in_code {
                    code_buf.push_str(&text);
                } else {
                    text_buf.push_str(&text);
                }
            }
            Event::Code(code) => {
                in_code = true;
                code_buf.push_str(&code);
                in_code = false;
            }
            Event::Html(html) => {
                text_buf.push_str(&html);
            }
            Event::SoftBreak => {
                text_buf.push('\n');
            }
            Event::HardBreak => {
                text_buf.push_str("  \n");
            }
            Event::Rule => {
                if let Some(ref mut open) = current_tag {
                    let open_kind = std::mem::replace(open, Tag::Paragraph(vec![]));
                    match open_kind.kind() {
                        TagEnd::Paragraph => {
                            if !text_buf.is_empty() {
                                children.push(Node::Text {
                                    content: std::mem::take(&mut text_buf),
                                    children: vec![],
                                });
                            }
                            children.push(Node::ThematicBreak);
                        }
                        _ => {
                            children.push(Node::ThematicBreak);
                        }
                    }
                } else {
                    children.push(Node::ThematicBreak);
                }
            }
        }
    }

    // Flush any remaining text
    if !text_buf.is_empty() {
        children.push(Node::Text {
            content: std::mem::take(&mut text_buf),
            children: vec![],
        });
    }
    if !code_buf.is_empty() {
        children.push(Node::CodeBlock {
            language: std::mem::take(&mut code_lang),
            code: std::mem::take(&mut code_buf),
        });
    }

    // If we have a remaining open tag, close it
    if let Some(open) = current_tag.take() {
        match open.kind() {
            TagEnd::Paragraph => {
                if !text_buf.is_empty() {
                    children.push(Node::Text {
                        content: std::mem::take(&mut text_buf),
                        children: vec![],
                    });
                }
            }
            _ => {}
        }
    }

    Ok(Node::Paragraph { children })
}

fn parse_table_children(
    children: Vec<Node>,
    alignments: &[pulldown_cmark::Alignment],
) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();

    for child in children {
        match child {
            Node::TableCell { children: cell_children } => {
                let cells: Vec<String> = cell_children
                    .iter()
                    .filter_map(|c| {
                        if let Node::Text { content, .. } = c {
                            Some(content.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let text = cells.join("");
                current_row.push(text);
            }
            Node::Table { .. } => {
                // Nested table or row separator consumed
            }
            _ => {}
        }
    }

    // First row is header
    if !current_row.is_empty() {
        headers = current_row.clone();
    }

    // Remaining rows
    rows = vec![current_row];

    (headers, rows)
}
