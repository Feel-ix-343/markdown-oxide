use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

use completions::QueryContext;
use diagnostics::diagnostics;
use itertools::Itertools;
use moxide_config::Settings;
use rayon::prelude::*;
use references::references;
use serde_json::Value;
use symbol::{document_symbol, workspace_symbol};
use tokio::sync::RwLock;

use gotodef::goto_definition;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};

use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use vault::Vault;

mod codeactions;
mod codelens;
mod commands;
mod daily;
mod diagnostics;
mod gotodef;
mod hover;
mod macros;
mod references;
mod rename;
mod symbol;
mod tokens;
mod ui;

#[derive(Debug)]
struct Backend {
    client: Client,
    vault: RwLock<Option<Vault>>,
    opened_files: RwLock<HashSet<PathBuf>>,
    settings: RwLock<Option<Settings>>,
    cache: Cache,
}
#[derive(Debug, Default)]
struct Cache {
    query_cache: RwLock<completions::QueryCache>,
}

struct TextDocumentItem {
    uri: Url,
    new_text: String,
    range: Option<Range>,
}

impl Backend {
    async fn update_vault(&self, params: TextDocumentItem) {
        self.client
            .log_message(MessageType::WARNING, "Update Vault Started")
            .await;

        let Ok(path) = params.uri.to_file_path() else {
            self.client
                .log_message(MessageType::ERROR, "Failed to parse URI path")
                .await;
            return;
        };

        let Ok(settings) = self.bind_settings(|settings| Ok(settings.clone())).await else {
            return;
        };

        let guard = self
            .bind_vault_mut(|vault| {
                let text = &params.new_text;
                Vault::update_vault(&settings, vault, (&path, text, params.range));

                Ok(())
            })
            .await;
        drop(guard);

        self.client
            .log_message(MessageType::WARNING, "Update Vault Done")
            .await;

        match self.publish_diagnostics().await {
            Ok(_) => (),
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Failed calculating diagnostics on vault update {:?}", e),
                    )
                    .await
            }
        }

        if settings.semantic_tokens {
            let _ = self.client.semantic_tokens_refresh().await;
        }
    }

    async fn reconstruct_vault(&self) {
        let progress = self
            .client
            .progress(ProgressToken::Number(1), "Constructing Vault")
            .begin()
            .await;

        let timer = std::time::Instant::now();

        let Ok(settings) = self.bind_settings(|settings| Ok(settings.clone())).await else {
            return;
        };

        {
            let _ = self
                .bind_vault_mut(|vault| {
                    let Ok(new_vault) = Vault::construct_vault(&settings, vault.root_dir()) else {
                        return Err(Error::new(ErrorCode::ServerError(0)));
                    };

                    *vault = new_vault;

                    Ok(())
                })
                .await;
        }

        let elapsed = timer.elapsed();

        progress
            .finish_with_message(format!("Finished in {}ms", elapsed.as_millis()))
            .await;

        if elapsed.as_millis() > 10 {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("Vault Construction took {}ms", elapsed.as_millis()),
                )
                .await;
        }

        match self.publish_diagnostics().await {
            Ok(_) => (),
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!(
                            "Failed calculating diagnostics on vault construction {:?}",
                            e
                        ),
                    )
                    .await
            }
        };

        let _ = self.client.semantic_tokens_refresh().await;
    }

    async fn publish_diagnostics(&self) -> Result<()> {
        let timer = std::time::Instant::now();

        self.client
            .log_message(MessageType::WARNING, "Diagnostics Started")
            .await;

        let uris = self
            .bind_opened_files(|files| {
                Ok(files
                    .into_par_iter()
                    .filter_map(|url| Url::from_file_path(url).ok())
                    .collect::<Vec<_>>())
            })
            .await?;

        let settings = self.bind_settings(|settings| Ok(settings.clone())).await?;

        let diagnostics = self
            .bind_vault(|vault| {
                Ok(uris
                    .par_iter()
                    .filter_map(|uri| {
                        let path = uri.to_file_path().ok()?;

                        diagnostics(vault, &settings, (&path, uri))
                            .map(|diags| (uri.clone(), diags))
                    })
                    .collect::<Vec<_>>())
            })
            .await?;

        for (uri, diags) in diagnostics {
            self.client.publish_diagnostics(uri, diags, None).await;
        }

        self.client
            .log_message(MessageType::WARNING, "Diagnostics Done")
            .await;

        let elapsed = timer.elapsed();

        self.client
            .log_message(
                MessageType::WARNING,
                format!("Diagnostics Done took {}ms", elapsed.as_millis()),
            )
            .await;

        Ok(())
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
    async fn bind_vault<T>(&self, callback: impl FnOnce(&Vault) -> Result<T>) -> Result<T> {
        let guard = self.vault.read().await;
        let Some(vault) = guard.deref() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };

        callback(vault)
    }

    async fn bind_vault_mut<T>(&self, callback: impl Fn(&mut Vault) -> Result<T>) -> Result<T> {
        if let Err(e) = self.vault.try_write() {
            self.client
                .log_message(
                    MessageType::ERROR,
                    format!("Failed to get VAULT lock for write {:?}", e),
                )
                .await;
        } else {
            self.client
                .log_message(MessageType::ERROR, "VAULT Lock is good")
                .await;
        }

        let mut guard = self.vault.write().await;
        let Some(ref mut vault) = *guard else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };

        callback(vault)
    }

    async fn bind_query_cx<T>(
        &self,
        callback: impl FnOnce(completions::QueryContext) -> Result<T>,
    ) -> Result<T> {
        let mut cache = self.cache.query_cache.write().await;
        let v_g = self.vault.read().await;
        let s_g = self.settings.read().await;
        let (cache, Some(vault), Some(settings)) = (cache.deref_mut(), v_g.deref(), s_g.deref())
        else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };
        let cx = QueryContext::new(vault, settings, cache);

        callback(cx)
    }

    async fn bind_settings<T>(&self, callback: impl FnOnce(&Settings) -> Result<T>) -> Result<T> {
        let guard = self.settings.read().await;
        let Some(settings) = guard.deref() else {
            return Err(Error::new(ErrorCode::ServerError(1)));
        };

        callback(settings)
    }

    async fn bind_opened_files<T>(
        &self,
        callback: impl Fn(&HashSet<PathBuf>) -> Result<T>,
    ) -> Result<T> {
        let opened_files = self.opened_files.read().await;
        callback(opened_files.deref())
    }

    async fn bind_opened_files_mut<T>(
        &self,
        callback: impl Fn(&mut HashSet<PathBuf>) -> Result<T>,
    ) -> Result<T> {
        if let Err(e) = self.opened_files.try_write() {
            self.client
                .log_message(
                    MessageType::ERROR,
                    format!("Failed to get FILES lock for write {:?}", e),
                )
                .await;
        } else {
            self.client
                .log_message(MessageType::ERROR, "FILES Lock is good")
                .await;
        }

        let mut opened_files = self.opened_files.write().await;
        callback(opened_files.deref_mut())
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, i: InitializeParams) -> Result<InitializeResult> {
        let root_dir = match i.root_uri {
            Some(uri) => uri
                .to_file_path()
                .or(Err(Error::new(ErrorCode::InvalidParams)))?,
            None => std::env::current_dir().or(Err(Error::new(ErrorCode::InvalidParams)))?,
        };

        let read_settings = match Settings::new(&root_dir, &i.capabilities) {
            Ok(settings) => settings,
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Failed to read settings {:?}", e),
                    )
                    .await;
                return Err(Error::new(ErrorCode::ServerError(1)));
            }
        };

        let Ok(vault) = Vault::construct_vault(&read_settings, &root_dir) else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };
        let mut value = self.vault.write().await;
        *value = Some(vault);

        let mut settings = self.settings.write().await;
        *settings = Some(read_settings);

        let file_op_reg = FileOperationRegistrationOptions {
            filters: std::iter::once(FileOperationFilter {
                pattern: FileOperationPattern {
                    options: None,
                    glob: "**/*.md".into(),
                    matches: None,
                },
                ..Default::default()
            })
            .collect(),
        };

        return Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        "[".into(),
                        " ".into(),
                        "(".into(),
                        "#".into(),
                        ">".into(),
                    ]),
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
                workspace: Some(WorkspaceServerCapabilities {
                    file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                        did_create: Some(file_op_reg.clone()),
                        did_rename: Some(file_op_reg.clone()),
                        did_delete: Some(file_op_reg.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["apply_edits".into(), "jump".into()],
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::DECORATOR,
                                    SemanticTokenType::COMMENT,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                    SemanticTokenModifier::DEPRECATED,
                                ],
                            },
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
        });
    }

    async fn shutdown(&self) -> Result<()> {
        // TODO: remove all code lenses
        std::process::exit(0);
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let path = params_path!(params)?;

        self.bind_vault(|vault| Ok(codelens::code_lens(vault, &path, &params)))
            .await
    }

    async fn initialized(&self, _: InitializedParams) {
        let settings = self
            .bind_settings(|settings| Ok(settings.clone()))
            .await
            .unwrap();
        self.client
            .log_message(MessageType::WARNING, format!("Settings: {:?}", settings))
            .await;

        let Ok(root_path) = self.bind_vault(|vault| Ok(vault.root_dir().clone())).await else {
            return;
        };

        let Ok(_root_uri) = Url::from_directory_path(root_path) else {
            return;
        };

        let value = serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
            watchers: vec![FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.md".into()),
                kind: None,
            }],
        })
        .unwrap();

        let registration = Registration {
            id: "myserver-fileWatcher".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(value),
        };

        self.client
            .register_capability(vec![registration])
            .await
            .unwrap();
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        {
            let _new_files = self
                .bind_opened_files_mut(|files| {
                    // diagnostics will only be published for the files that are opened; We must track which files are opened
                    let path = params_path!(params)?;

                    files.insert(path);

                    Ok(files.clone())
                })
                .await;

            self.client
                .log_message(MessageType::LOG, "Added file")
                .await;

            self.update_vault(TextDocumentItem {
                uri: params.text_document.uri,
                new_text: params.text_document.text,
                range: None,
            })
            .await; // usually, this is not necesary; however some may start the LS without saving a changed file, so it is necessary
        } // drop the lock

        match self.publish_diagnostics().await {
            Ok(_) => (),
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Failed calculating diagnostics on file open {:?}", e),
                    )
                    .await
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let removed_file = self
            .bind_opened_files_mut(|files| {
                let path = params_path!(params)?;

                Ok(files.take(&path))
            })
            .await;

        if let Ok(Some(file)) = removed_file {
            self.client
                .log_message(MessageType::LOG, format!("Remove file {:?}", file))
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        for change in &params.content_changes {
            self.update_vault(TextDocumentItem {
                uri: params.text_document.uri.clone(),
                new_text: change.text.clone(),
                range: change.range,
            })
            .await;
        }

        let cache_result = self
            .bind_query_cx(|cx| Ok(completions::lsp_sync(cx, params)))
            .await;
        self.client
            .log_message(
                MessageType::INFO,
                format!("Cache Result: {:?}", cache_result),
            )
            .await;
    }

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {
        self.reconstruct_vault().await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.bind_vault(|vault| {
            let path = params_path!(params.text_document_position_params)?;
            Ok(
                goto_definition(vault, params.text_document_position_params.position, &path)
                    .map(GotoDefinitionResponse::Array),
            )
        })
        .await
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        self.bind_vault(|vault| {
            let path = params_position_path!(params)?;
            Ok(references(
                vault,
                params.text_document_position.position,
                &path,
            ))
        })
        .await
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.client
            .log_message(MessageType::WARNING, "Completions Started")
            .await;

        let timer = std::time::Instant::now();

        let path = params_position_path!(params)?;
        let files = self
            .bind_opened_files(|files| Ok(files.clone().into_iter().collect::<Box<[_]>>()))
            .await?;

        let Ok(settings) = self.bind_settings(|settings| Ok(settings.to_owned())).await else {
            return Err(Error::new(ErrorCode::ServerError(2)));
        }; // TODO: this is bad

        let res = self
            .bind_query_cx(|cx| Ok(completions::get_completions(&params, &path, cx)))
            .await;

        let elapsed = timer.elapsed();

        self.client
            .log_message(
                MessageType::WARNING,
                format!("Completions Done took {}ms", elapsed.as_millis()),
            )
            .await;

        res
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        match params {
            ExecuteCommandParams { command, .. } if *command == *"apply_edits" => {
                let edits = params
                    .arguments
                    .into_iter()
                    .filter_map(|arg| serde_json::from_value::<WorkspaceEdit>(arg).ok())
                    .collect_vec();

                for edit in edits {
                    let _ = self.client.apply_edit(edit).await;
                }

                Ok(None)
            }
            ExecuteCommandParams { command, .. } if *command == *"jump" => {
                let jump_to = params.arguments.first().and_then(|val| val.as_str());
                let settings = self
                    .bind_settings(|settings| Ok(settings.to_owned()))
                    .await?;
                let root_dir = self
                    .bind_vault(|vault| Ok(vault.root_dir().to_owned()))
                    .await?;
                commands::jump(&self.client, &root_dir, &settings, jump_to).await
            }
            _ => Ok(None),
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.bind_vault(|vault| {
            let path = params_path!(params.text_document_position_params)?;
            Ok(hover::hover(vault, &params, &path))
        })
        .await
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        self.bind_vault(|vault| {
            let path = params_path!(params)?;
            Ok(document_symbol(vault, &params, &path))
        })
        .await
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        self.bind_vault(|vault| Ok(workspace_symbol(vault, &params)))
            .await
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        self.bind_vault(|vault| {
            let path = params_position_path!(params)?;
            Ok(rename::rename(vault, &params, &path))
        })
        .await
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let settings = self.bind_settings(|settings| Ok(settings.clone())).await?;

        self.bind_vault(|vault| {
            let path = params_path!(params)?;
            Ok(codeactions::code_actions(vault, &params, &path, &settings))
        })
        .await
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let settings = self.bind_settings(|settings| Ok(settings.clone())).await?;

        let timer = std::time::Instant::now();

        let path = params_path!(params)?;
        let res = self
            .bind_vault(|vault| {
                Ok(tokens::semantic_tokens_full(
                    vault, &path, params, &settings,
                ))
            })
            .await;

        let elapsed = timer.elapsed();

        self.client
            .log_message(
                MessageType::WARNING,
                format!("Semantic Tokens Done took {}ms", elapsed.as_millis()),
            )
            .await;

        return res;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        vault: None.into(),
        opened_files: HashSet::new().into(),
        settings: None.into(),
        cache: Cache::default(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
