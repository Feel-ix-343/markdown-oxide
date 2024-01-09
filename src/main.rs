use std::ops::Deref;
use std::path::Path;

use completion::get_completions;
use diagnostics::diagnostics;
use references::references;
use tokio::sync::RwLock;

use gotodef::goto_definition;
use tower_lsp::jsonrpc::{Result, Error, ErrorCode};
use tower_lsp::lsp_types::*;
use tower_lsp::lsp_types::notification::{Notification, Progress};
use tower_lsp::{Client, LanguageServer, LspService, Server};
use vault::Vault;

mod vault;
mod gotodef;
mod references;
mod completion;
mod diagnostics;
mod hover;
mod ui;


#[derive(Debug)]
struct Backend {
    client: Client,
    vault: RwLock<Option<Vault>>
}


struct TextDocumentItem {
    uri: Url,
    text: String,
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let Some(ref mut vault) = *self.vault.write().await else {
            self.client.log_message(MessageType::ERROR, "Vault is not initialized").await;
            return;
        };

        let Ok(path) = params.uri.to_file_path() else {
            self.client.log_message(MessageType::ERROR, "Failed to parse URI path").await;
            return;
        };
        let text = &params.text;
        Vault::reconstruct_vault(vault, (&path, text));

        diagnostics(&vault, (&path, &params.uri, text), &self.client).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, i: InitializeParams) -> Result<InitializeResult> {
        let Some(root_uri) = i.root_uri else {
            return Err(Error::new(ErrorCode::InvalidParams));
        };
        let root_dir = Path::new(root_uri.path());
        let Ok(vault) = Vault::construct_vault(root_dir) else {
            return Err(Error::new(ErrorCode::ServerError(0)))
        };
        let mut value = self.vault.write().await;
        *value = Some(vault);


        return Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec!["[".into()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                // definition: Some(GotoCapability::default()),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            }
        })
    }

    

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.send_notification::<Progress>(ProgressParams {
            token: NumberOrString::Number(1),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd { message: Some("Initialized".into()) }))
        }).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentItem {uri: params.text_document.uri, text: params.text_document.text}).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem { uri: params.text_document.uri, text: params.content_changes.remove(0).text }).await;
    }



    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {

        let position = params.text_document_position_params.position;

        let vault_option = self.vault.read().await;
        let Some(vault) = vault_option.deref() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };
        let Ok(path) = params.text_document_position_params.text_document.uri.to_file_path() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };
        let result = goto_definition(&vault, position, &path);


        return Ok(result.map(|l| GotoDefinitionResponse::Array(l)))
    }


    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let position = params.text_document_position.position;

        let vault_option = self.vault.read().await;
        let Some(vault) = vault_option.deref() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };
        let Ok(path) = params.text_document_position.text_document.uri.to_file_path() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };

        let locations = references(vault, position, &path);
        Ok(locations)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.client.send_notification::<Progress>(ProgressParams {
            token: NumberOrString::Number(2),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin { 
                title: "Getting Completions".into(), 
                cancellable: None, 
                message: None, 
                percentage: None })
            )
        }).await;

        let bad_vault = self.vault.read().await;
        let Some(vault) = bad_vault.deref() else {
            return Err(Error::new(ErrorCode::ServerError(0)))
        };
        let completions = get_completions(vault, &params);
        if completions == None {
            self.client.send_notification::<Progress>(ProgressParams {
                token: NumberOrString::Number(2),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd { message: Some("No Completions".into()) }))
            }).await;
        }

        self.client.send_notification::<Progress>(ProgressParams {
            token: NumberOrString::Number(2),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd { message: Some("Got Completions".into()) }))
        }).await;

        Ok(completions)

    }


    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let bad_vault = self.vault.read().await;
        let Some(vault) = bad_vault.deref() else {
            return Err(Error::new(ErrorCode::ServerError(0)))
        };
        let Ok(path) = params.text_document_position_params.text_document.uri.to_file_path() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };
        return Ok(hover::hover(&vault, params, &path))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client, vault: None.into() });
    Server::new(stdin, stdout, socket).serve(service).await;
}
