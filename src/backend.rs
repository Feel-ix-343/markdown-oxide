use std::path::PathBuf;

use tower_lsp::{
    lsp_types::{
        notification::{Notification, PublishDiagnostics},
        request::{GotoDefinition, Request},
        *,
    },
    Client,
};
use url::Url;

use crate::{
    config::Settings,
    lsp::{
        goto_definition::goto_definition,
        references::find_references,
        semantic_tokens::semantic_tokens,
        symbols::workspace_symbol,
        util::{get_uri, path_from_url},
    },
    vault::Vault,
};

pub struct Backend {
    pub client: Client,
    pub vault: Vault,
    pub settings: Settings,
    pub opened_files: Vec<PathBuf>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: None,
                    trigger_characters: Some(vec![
                        String::from("#"),
                        String::from("["),
                        String::from("]"),
                        String::from("("),
                        String::from("<"),
                        String::from("!"),
                        String::from("^"),
                    ]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: Some(CompletionItemCapability {
                        snippet_support: Some(true),
                        ..Default::default()
                    }),
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["markdown-oxide.copyMarkdownUrl".to_string()],
                    work_done_progress_options: Default::default(),
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                            legend: SemanticTokensLegend {
                                token_types: semantic_tokens::get_token_types(),
                                token_modifiers: semantic_tokens::get_token_modifiers(),
                            },
                            range: None,
                            full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                        },
                    ),
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let path = path_from_url(&params.text_document.uri).unwrap();
        self.vault.update_file(&path).await;
        self.client
            .publish_diagnostics(
                params.text_document.uri,
                vec![],
                Some(params.text_document.version),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let path = path_from_url(&params.text_document.uri).unwrap();
        self.vault.update_file(&path).await;
        self.client
            .publish_diagnostics(
                params.text_document.uri,
                vec![],
                Some(params.text_document.version),
            )
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let path = path_from_url(&params.text_document.uri).unwrap();
        self.vault.remove_file(&path).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for change in params.changes {
            let path = path_from_url(&change.uri).unwrap();
            self.vault.update_file(&path).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let path = path_from_url(&params.text_document_position.text_document.uri)?;
        let completions = crate::completion::get_completions(
            &self.vault,
            &self.opened_files,
            &params,
            &path,
            &self.settings,
        );

        Ok(completions)
    }

    async fn symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let path = path_from_url(&params.text_document.uri)?;
        let symbols = crate::lsp::symbols::document_symbols(&self.vault, &path);
        Ok(symbols)
    }

    async fn workspace_symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<WorkspaceSymbolResponse>> {
        let symbols = workspace_symbol(&self.vault, &params.query);
        Ok(symbols)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let path = path_from_url(&params.text_document_position_params.text_document.uri)?;
        let definition = goto_definition(&self.vault, &path, &params.text_document_position_params.position);
        Ok(definition)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let path = path_from_url(&params.text_document_position.text_document.uri)?;
        let references = find_references(&self.vault, &path, &params.text_document_position.position);
        Ok(references)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensFullParams,
    ) -> Result<Option<SemanticTokensFullResult>> {
        let path = path_from_url(&params.text_document.uri)?;
        let tokens = semantic_tokens(&self.vault, &path);
        Ok(tokens)
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<JsonValue>> {
        match params.command.as_str() {
            "markdown-oxide.copyMarkdownUrl" => {
                let path = path_from_url(
                    &params
                        .arguments
                        .get(0)
                        .and_then(|v| v.as_str())
                        .map(Url::parse)
                        .transpose()?
                        .ok_or_else(|| {
                            Error::invalid_params("First argument must be a valid URI")
                        })?,
                )?;
                let alias = params
                    .arguments
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let url = get_uri(&self.vault, &path, alias);
                self.client
                    .apply_edit(WorkspaceEdit::default().with_document_changes(vec![
                        DocumentChangeOperation::Op(ResourceOp::new(
                            url,
                            OperationKind::Create,
                            None,
                        )),
                    ]))
                    .await?;
                Ok(Some(JsonValue::String(url.to_string())))
            }
            _ => Err(Error::invalid_params("Unknown command")),
        }
    }
}