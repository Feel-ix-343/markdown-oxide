use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod vault;
mod gotodef;


#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }


    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}

//
// #[tokio::main]
// fn main() {
//
//     let a = Analysis::new("/home/felix/Notes").unwrap();
//     let completion = "felix";
//     println!("Completions for {completion}: {:#?}", a.get_link_completions(completion));
//     //
//     // // Get incoming for "Obsidian Text Link Suggestions"
//     //
//     // println!("References: File: Obsidian Text Link Suggestions {:#?}", a.file_incoming("Obsidian Text Link Suggestions"));
//     // println!("References: File: Practice Reflections.md {:#?}", a.file_incoming("Practice Reflections"));
//     // println!("References: Heading: cons in Practice Reflections {:#?}", a.heading_incoming("Practice Reflections", "cons"));
//     // println!("References: Tag: #MapOfContent/apworld: {:#?}", a.tags_incoming("MapOfContent/aplit"));
//
//     // let analyzer = Analyzer::new("/home/felix/Notes");
//     // let graph = analyzer.construct_graph();
//
//     // // test 
//     // let current_file_path: PathBuf = PathBuf::from("/home/felix/Notes/Obsidian Markdown Language Server.md");
//     // let md_file: &MDFile = analyzer.files.get(&current_file_path).unwrap();
//     // let incoming_links = md_file.incoming(&graph).unwrap();
//     // let names = incoming_links.iter().map(|f| f.name()).collect_vec();
//     // println!("Incoming {:#?}", names);
//     // let outgoing_links = md_file.outgoing(&graph).unwrap();
//     // let names = outgoing_links.iter().map(|f| f.name()).collect_vec();
//     // println!("Outgoing {:#?}", names);
//
//     // // Test heading
//
//     // let heading = md_file.headings.iter().find(|h| h.ref_name == "Obsidian Markdown Language Server#Development").unwrap();
//     // let heading_incoming = heading.incoming(&graph).unwrap();
//     // let names = heading_incoming.iter().map(|f| f.name()).collect_vec();
//     // println!("Heading Incoming {:#?}", names);
// }
//
//
