#![feature(slice_split_once)]

use std::ops::Deref;
use std::path::{Path, PathBuf};

use completion::get_completions;
use diagnostics::diagnostics;
use references::references;
use symbol::{document_symbol, workspace_symbol};
use tokio::sync::RwLock;

use gotodef::goto_definition;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};

use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use vault::Vault;

mod completion;
mod diagnostics;
mod gotodef;
mod hover;
mod references;
mod rename;
mod symbol;
mod ui;
mod vault;
mod codeactions;
mod macros;

#[derive(Debug)]
struct Backend {
    client: Client,
    vault: RwLock<Option<Vault>>,
}

struct TextDocumentItem {
    uri: Url,
    text: String,
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let Some(ref mut vault) = *self.vault.write().await else {
            self.client
                .log_message(MessageType::ERROR, "Vault is not initialized")
                .await;
            return;
        };

        let Ok(path) = params.uri.to_file_path() else {
            self.client
                .log_message(MessageType::ERROR, "Failed to parse URI path")
                .await;
            return;
        };
        let text = &params.text;
        Vault::reconstruct_vault(vault, (&path, text));

        diagnostics(vault, (&path, &params.uri, text), &self.client).await;
    }


    /// This is an FP reference. Lets say that there is monad around the vault of type Result<Vault>, representing accesing the RwLock arond it in async
    /// This function will extract the vautl result, apply the given function which will return another monad (which I am asuming to be another result)
    /// The function then returns this monad
    ///
    /// I think this is a nice pattern; convienient and pretty simple api; cool stuff, if I say so myself!
    ///
    /// TODO: Hopefully rust async closures will be more convienient to use eventually and this can accept an async closure; this would enable better logging
    /// in the call back functions. (though to get aroudn this, the callback could return a Result of a writer style monad, which could be logged async outside of
    /// the callback)
    async fn bind_vault<T>(&self, callback: impl Fn(&Vault) -> Result<T>) -> Result<T>
    {

        let vault_option = self.vault.read().await;
        let Some(vault) = vault_option.deref() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };

        return callback(&vault)
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
            return Err(Error::new(ErrorCode::ServerError(0)));
        };
        let mut value = self.vault.write().await;
        *value = Some(vault);

        let file_op_reg = FileOperationRegistrationOptions{
            filters: std::iter::once(
                FileOperationFilter {
                    pattern: FileOperationPattern {
                        options: None,
                        glob: "**/*.md".into(),
                        matches: None
                    },
                    ..Default::default()
                }
            ).collect()
        } ;


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
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                workspace: Some(WorkspaceServerCapabilities{
                    file_operations: Some(WorkspaceFileOperationsServerCapabilities{
                        did_create: Some(file_op_reg.clone()),
                        did_rename: Some(file_op_reg.clone()),
                        did_delete: Some(file_op_reg.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        });
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
        })
        .await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.content_changes.remove(0).text,
        })
        .await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.bind_vault(|vault| {
            let path = params_path!(params.text_document_position_params)?;
            Ok(goto_definition(vault, params.text_document_position_params.position, &path).map(GotoDefinitionResponse::Array))
        }).await
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        self.bind_vault(|vault| {
            let path = params_position_path!(params)?;
            Ok(references(vault, params.text_document_position.position, &path))
        }).await
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let progress = self
            .client
            .progress(ProgressToken::Number(1), "Calculating Completions")
            .begin()
        .await;
        let timer = std::time::Instant::now();

        let res = self.bind_vault(|vault| {
            Ok(get_completions(vault, &params))
        }).await;

        let elapsed = timer.elapsed();

        progress
            .finish_with_message(format!("Finished in {}ms", elapsed.as_millis()))
        .await;

        if elapsed.as_millis() > 10  {
            self.client.log_message(MessageType::WARNING, format!("Completion Calculation took a long time: Finished in {}ms", elapsed.as_millis())).await;
        }


        res
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.bind_vault(|vault| {
            let path = params_path!(params.text_document_position_params)?;
            return Ok(hover::hover(vault, &params, &path));
        }).await
    }

    async fn document_symbol( &self, params: DocumentSymbolParams,) -> Result<Option<DocumentSymbolResponse>> {
        self.bind_vault(|vault| {
            let path = params_path!(params)?;
            return Ok(document_symbol(vault, &params, &path));
        }).await
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        self.bind_vault(|vault| {
            return Ok(workspace_symbol(vault, &params));
        }).await
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        self.bind_vault(|vault| {
            let path = params_position_path!(params)?;
            return Ok(rename::rename(vault, &params, &path));
        }).await
    }


    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        self.bind_vault(|vault| {
            let path = params_path!(params)?;
            return Ok(codeactions::code_actions(vault, &params, &path))
        }).await
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        vault: None.into(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
