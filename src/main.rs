use tree_sitter::QueryCursor;
use::tree_sitter_md::{MarkdownParser};
use::tree_sitter_md::{language, inline_language};
use::tree_sitter::{Query, TextProvider};

fn main() {
    let mut parser = MarkdownParser::default();

    // Read the ../TestFiles/Test.md
    let test_md_file = std::fs::read("./TestFiles/Test.md").unwrap();

    let source_code: &[u8] = &test_md_file;
    let tree = parser.parse(source_code, None).unwrap();
    let language = language();
    let inline_language = inline_language();

    let block_tree = tree.block_tree();
    let inline_trees = tree.inline_trees();

    // println!("{:?}", block_tree.root_node().to_sexp());
    // inline_trees.iter().for_each(|node| println!("{:?}", node.root_node().to_sexp()));

    // Finding links in the files
    

    // Execute a treesitter query for finding the links from a file
    let query = Query::new(inline_language, "(link_text) @link;").unwrap();
    let mut query_cursor = QueryCursor::new();
    let text_provider: &[u8] = &[];
    let links: Vec<&str> = inline_trees
        .iter()
        .flat_map(|tree| {
            let matches = query_cursor.captures(&query, tree.root_node(), text_provider);
            let links: Vec<&str> = matches
                .flat_map(|(q, _)|
                    q.captures
                        .iter()
                        .map(|c| c.node.utf8_text(source_code).unwrap())
                )
                .collect();
            return links
        })
        .collect();

    // Pring the matches
    println!("LOOK HERE; the links in the file:\n{:#?}", links)

}
