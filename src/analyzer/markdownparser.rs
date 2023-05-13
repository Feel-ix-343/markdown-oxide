use itertools::Itertools;
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{MarkdownParser, inline_language, language};


pub struct MarkdownNodeParser {
    parser: MarkdownParser,
}

impl MarkdownNodeParser {
    pub fn new() -> MarkdownNodeParser {
        MarkdownNodeParser {
            parser: MarkdownParser::default()
        }
    }

    fn query_matches_inline<'a>(&mut self, source_code: &'a [u8], query: &str) -> Vec<&'a str> {
        let parser = &mut self.parser;

        let tree = parser.parse(source_code, None).unwrap();
        // let language = language();
        let inline_language = inline_language();

        let inline_trees = tree.inline_trees();

        // Finding links in the files

        // Execute a treesitter query for finding the links from a file
        let query = Query::new(inline_language, query).unwrap();

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

    fn query_matches_block<'a>(&mut self, source_code: &'a [u8], query: &str) -> Vec<&'a str> {
        let parser = &mut self.parser;

        let tree = parser.parse(source_code, None).unwrap();
        // let language = language();
        let language = language();

        let block_tree = tree.block_tree();

        // Finding links in the files

        // Execute a treesitter query for finding the links from a file
        let query = Query::new(language, query).unwrap();

        let mut query_cursor = QueryCursor::new();
        let text_provider: &[u8] = &[];

        let text = query_cursor
            .matches(&query, block_tree.root_node(), text_provider)
            .flat_map(|m| m.captures)
            .map(|c| c.node.utf8_text(source_code).unwrap())
            .collect_vec();

        // wow what a bad bug! Don't do this! All of the matches will have the ranges
        // let matches = query_cursor.matches(&query, block_tree.root_node(), text_provider).collect_vec();
        // println!("matches: {:?}", matches);

        return text
    }

    pub fn links_for_file<'a>(&mut self, source_code: &'a [u8]) -> Vec<&'a str> {
        return self.query_matches_inline(source_code, "(link_text) @link;");
    }


    pub fn headings_for_file<'a>(&mut self, source_code: &'a [u8]) -> Vec<&'a str> {
        let headings = self.query_matches_block(source_code, "(atx_heading heading_content: (inline) @link);");
        let trimmed_headings = headings.iter().map(|s| s.trim_start()).collect_vec();
        println!("headings: {:?}", trimmed_headings);
        return trimmed_headings
    }
}
