use std::path::PathBuf;

use tree_sitter::QueryCursor;
use::tree_sitter_md::{MarkdownParser};
use::tree_sitter_md::{language, inline_language};
use::tree_sitter::{Query, TextProvider};
use::itertools::Itertools;
use::rayon::prelude::*;

mod analyzer;
mod lsp;

use analyzer::*;

fn main() {
    let analyzer = Analyzer::new("/home/felix/Notes");
    let graph = analyzer.construct_graph();
}


