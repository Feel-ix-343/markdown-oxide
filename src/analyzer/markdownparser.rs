use itertools::Itertools;
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{MarkdownParser, inline_language};


pub struct MarkdownLinkParser {
    parser: MarkdownParser,
}

impl MarkdownLinkParser {
    pub fn new() -> MarkdownLinkParser {
        MarkdownLinkParser {
            parser: MarkdownParser::default()
        }
    }

    pub fn links_for_file<'a>(&mut self, source_code: &'a [u8]) -> Vec<&'a str> {
        let parser = &mut self.parser;

        let tree = parser.parse(source_code, None).unwrap();
        // let language = language();
        let inline_language = inline_language();

        // let block_tree = tree.block_tree();
        let inline_trees = tree.inline_trees();

        // Finding links in the files

        // Execute a treesitter query for finding the links from a file
        let query = Query::new(inline_language, "(link_text) @link;").unwrap();

        let mut query_cursor = QueryCursor::new();
        let text_provider: &[u8] = &[];

        let links: Vec<&str> = inline_trees // There are multiple inline trees
            .iter() // Iterate over each of them
            .flat_map(|tree| {
                let captures = query_cursor.captures(&query, tree.root_node(), text_provider).collect_vec();
                return captures.into_iter().flat_map(|(q, _)| q.captures).map(|c| c.node.utf8_text(source_code).unwrap()).collect_vec()
            }) // Map each tree to its query captures, then flatten all trees to a collection of their query captures
            .collect(); // TODO: I still want to refactor this more

        return links
    }
}
