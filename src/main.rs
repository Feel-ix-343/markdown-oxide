use tree_sitter::QueryCursor;
use::tree_sitter_md::{MarkdownParser};
use::tree_sitter_md::{language, inline_language};
use::tree_sitter::{Query, TextProvider};
use::itertools::Itertools;

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

    let links: Vec<&str> = inline_trees // There are multiple inline trees
        .iter() // Iterate over each of them
        .flat_map(|tree| {
            let mut query_cursor = QueryCursor::new();
            let text_provider: &[u8] = &[];
            let captures = query_cursor.captures(&query, tree.root_node(), text_provider).collect_vec();
            return captures.into_iter().flat_map(|(q, _)| q.captures).map(|c| c.node.utf8_text(source_code).unwrap()).collect_vec()
        }) // Map each tree to its query captures, then flatten all trees to a collection of their query captures
        .collect(); // TODO: I still want to refactor this more

    // Pring the matches
    println!("LOOK HERE; the links in the file:\n{:#?}", links)

}
