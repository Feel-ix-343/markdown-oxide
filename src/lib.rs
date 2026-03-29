// ... (bestehende Imports)
mod uri;
mod handlers;

// ... (bestehender Code in main.rs)

impl Default for ServerState {
    fn default() -> Self {
        Self {
            workspace_root: std::env::current_dir().unwrap_or_default(),
            // ... (bestehende Felder)
        }
    }
}

// ... (bestehende Server-Implementierung)

#[tower_lsp::async_trait]
impl LanguageServer for MarkdownOxide {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            if let Ok(root_path) = root_uri.to_file_path() {
                self.state.lock().await.workspace_root = root_path;
            }
        }

        // ... (bestehender Code)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) -> Result<()> {
        handlers::handle_did_open(&mut self.state.lock().await, params)
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) -> Result<()> {
        handlers::handle_did_change(&mut self.state.lock().await, params)
    }

    // ... (bestehende Handler)
}