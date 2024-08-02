use tree_sitter_md::MarkdownParser;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[derive(Debug)]
struct Document {
    nodes: Vec<Block>,
}

#[derive(Debug)]
enum Node {
    Block(Block),
    Heading(Heading),
}

#[derive(Debug)]
enum Block {
    ListBlock(ListBlock),
    ParagraphBlock(ParagraphBlock),
}

#[derive(Debug)]
struct ListBlock {
    children: Vec<ListBlock>,
}

#[derive(Debug)]
struct ParagraphBlock;

#[derive(Debug)]
struct Heading {
    children: Vec<Node>,
}

/// Parse text and return s expression
fn parse(file_text: &str) -> Option<Document> {
    let mut markdown_parser = MarkdownParser::default();
    let tree = markdown_parser.parse(file_text.as_bytes(), None)?;

    println!("--------------");

    let mut cursor = tree.walk();
    dbg!(cursor.goto_first_child());
    dbg!(cursor.node().kind());
    dbg!(cursor.goto_first_child());
    dbg!(cursor.node().kind());
    dbg!(cursor.is_inline());
    dbg!(cursor.goto_first_child()); // go into the paragraph
    dbg!(cursor.is_inline());
    dbg!(cursor.node().kind());
    dbg!(cursor.node()); // go into the inline node
    dbg!(cursor.goto_first_child());
    dbg!(cursor.node());
    dbg!(cursor.goto_next_sibling());
    dbg!(cursor.node());
    dbg!(cursor.goto_next_sibling());
    dbg!(cursor.node());
    dbg!(cursor.goto_next_sibling());
    dbg!(cursor.node());
    dbg!(cursor.goto_next_sibling());
    dbg!(cursor.node());
    dbg!(cursor.goto_next_sibling());
    dbg!(cursor.node());

    println!("--------------");

    dbg!(tree.walk().node().to_sexp());

    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let file_text = r#"
Make a *function* for tree-sitter to work with rust well #LATER more text [[Link#HEad]]

This is another task #LATER

# Task Overview

- Things that I want to keep planned:
    - [ ] ![[file#^infileref]]
    - ![[file#^infileref12]]
    - ![[file#^infileref123]]
    - Task defined here #LATER
- Things that I do not want to keep planned:
    - ![[differentfile#^blockref]]"#;

        println!("{:#?}", parse(file_text).unwrap())

        // assert_eq!(file_text, "How will this print?");
    }
}
