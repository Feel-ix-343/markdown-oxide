use analyzer::analyzer::Analyzer;

mod analyzer;
mod lsp;

fn main() {
    let analyzer = Analyzer::new("/home/felix/Notes");
    let graph = analyzer.construct_graph();
}


