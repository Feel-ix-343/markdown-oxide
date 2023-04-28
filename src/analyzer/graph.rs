use std::path::PathBuf;

use itertools::Itertools;
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{MarkdownParser, inline_language};

// NODE: file, headings, tags, ... ... maybe highlighted text, prob different though

// NOTES: Functionality
// 1. Get incoming references for all node types
// 2. Get outgoing references from current file

// Potential Applications (in no order)
// - Renaming node renames in all incoming
// - Go to definition for nodes
// - References for a node
// - Transform to node suggestions

trait Node {
    fn incoming(&self) -> Vec<&Self>;
    fn outgoing(&self) -> Vec<&Self>;
}

// for a file: parse for nodes -> calculate incoming and outgoing for each node
// ex: parse for headings and in-file-references -> calculate which refs are to headings (assuming that no headings have references)

// list of all nodes -> iter through each -> attach the outgoing links and link to the outgoing link's incoming link

