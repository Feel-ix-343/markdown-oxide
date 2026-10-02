```rust
use markdown;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::Serialize;
use std::collections::HashMap;

mod ast;
mod parse;
mod render;
mod resolve;

pub use ast::*;
pub use parse::*;
pub use render::*;
pub use resolve::*;

/// Parse a markdown string into an AST
pub fn parse(markdown: &str) -> Result<Document, String> {
    parse::parse_document(markdown)
}

/// Render an AST back to markdown
pub fn render(document: &Document) -> String {
    render::render_document(document)
}

/// Resolve references in a document
pub fn resolve(document: &Document) -> Result<Document, Vec<String>> {
    resolve::resolve_document(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_brackets_in_text() {
        let markdown = "* DO NOT use the square bracket `[[` and `]]` markers";
        let doc = parse(markdown).unwrap();
        // Should not produce any unresolved reference errors
        let result = resolve(&doc);
        assert!(result.is_ok(), "Should not have unresolved references for isolated [[ and ]]");
    }
}
