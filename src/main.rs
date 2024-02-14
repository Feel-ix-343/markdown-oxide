#![feature(slice_split_once)]
#![feature(async_closure)]

use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use completion::get_completions;
use diagnostics::diagnostics;
use itertools::Itertools;
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
mod completion;
mod diagnostics;
mod gotodef;
mod hover;
mod macros;
mod references;
mod rename;
mod symbol;
mod ui;
mod vault;

#[derive(Debug)]
struct Backend {
    client: Client,
    vault: RwLock<Option<Vault>>,
    opened_files: RwLock<HashSet<PathBuf>>,
}

struct TextDocumentItem {
    uri: Url,
    text: String,
}

impl Backend {
    async fn update_vault(&self, params: TextDocumentItem) {
        {
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
            Vault::update_vault(vault, (&path, text));
        } // must close the write lock before publishing diagnostics; I don't really like how imperative this is; TODO: Fix this shit

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
    }

    async fn reconstruct_vault(&self) {
        let progress = self
            .client
            .progress(ProgressToken::Number(1), "Constructing Vault")
            .begin()
            .await;

        let timer = std::time::Instant::now();

        {
            let _ = self
                .bind_vault_mut(|vault| {
                    let Ok(new_vault) = Vault::construct_vault(vault.root_dir()) else {
                        return Err(Error::new(ErrorCode::ServerError(0)));
                    };

                    *vault = new_vault;

                    Ok(())
                })
                .await;
        } // same issue as in the above function; TODO: Fix this

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
        }
    }

    async fn publish_diagnostics(&self) -> Result<()> {
        let urls = self.bind_opened_files(|files| Ok(files.clone())).await?;
        let uris = urls
            .into_iter()
            .filter_map(|url| Url::from_file_path(url).ok())
            .collect_vec();

        let diagnostics = self
            .bind_vault(|vault| {
                Ok(uris
                    .iter()
                    .filter_map(|uri| {
                        let path = uri.to_file_path().ok()?;

                        diagnostics(vault, (&path, &uri)).map(|diags| (uri.clone(), diags))
                    })
                    .collect_vec())
            })
            .await?;

        self.client
            .log_message(
                MessageType::LOG,
                format!(
                    "Calcualted Diagnostics for files: {:?}",
                    diagnostics.iter().map(|(uri, _)| uri).collect_vec()
                ),
            )
            .await;

        for (uri, diags) in diagnostics {
            self.client.publish_diagnostics(uri, diags, None).await;
        }

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
        let vault_option = self.vault.read().await;
        let Some(vault) = vault_option.deref() else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };

        callback(vault)
    }

    async fn bind_vault_mut<T>(&self, callback: impl Fn(&mut Vault) -> Result<T>) -> Result<T> {
        let Some(ref mut vault) = *self.vault.write().await else {
            return Err(Error::new(ErrorCode::ServerError(0)));
        };

        callback(vault)
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
        let mut opened_files = self.opened_files.write().await;
        callback(opened_files.deref_mut())
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
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec!["[".into(), " ".into()]),
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
                    commands: vec!["apply_edits".into()],
                    ..Default::default()
                }),
                ..Default::default()
                            
            },
        });
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let path = params_path!(params)?;

        self.bind_vault(|vault| Ok(codelens::code_lens(vault, &path, &params)))
            .await
    }

    async fn initialized(&self, _: InitializedParams) {
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
        let _ = self
            .bind_opened_files_mut(|files| {
                // diagnostics will only be published for the files that are opened; We must track which files are opened
                let path = params_path!(params)?;

                files.insert(path);

                Ok(())
            })
            .await;

        self.update_vault(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
        })
        .await; // usually, this is not necesary; however some may start the LS without saving a changed file, so it is necessary

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

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.update_vault(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.content_changes.remove(0).text,
        })
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
            .log_message(MessageType::LOG, "Getting Completions")
            .await;

        let progress = self
            .client
            .progress(ProgressToken::Number(1), "Calculating Completions")
            .begin()
            .await;

        let timer = std::time::Instant::now();

        let path = params_position_path!(params)?;
        let files = self.bind_opened_files(|files| Ok(files.clone().into_iter().collect::<Box<[_]>>())).await?;

        let res = self
            .bind_vault(|vault| Ok(get_completions(vault, &files, &params, &path)))
            .await;

        self.client.log_message(MessageType::LOG, format!("Completions: {:?}", res)).await;

        let elapsed = timer.elapsed();

        progress
            .finish_with_message(format!("Finished in {}ms", elapsed.as_millis()))
            .await;

        self.client
            .log_message(
                MessageType::WARNING,
                format!("Completion Calculation took {}ms", elapsed.as_millis()),
            )
            .await;

        res
    }


    async fn completion_resolve(&self, params: CompletionItem) -> Result<CompletionItem> {
        completion::resolve_completion(&params, &self.client).await
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        match params {
            ExecuteCommandParams { command, .. }  if *command == *"apply_edits" => {
                let edits = params.arguments.into_iter().map(|arg| serde_json::from_value::<WorkspaceEdit>(arg).ok()).flatten().collect_vec();

                for edit in edits {
                    let _ = self.client.apply_edit(edit).await;
                }

                Ok(None)
            },
            _ => Ok(None)
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
        self.bind_vault(|vault| {
            let path = params_path!(params)?;
            Ok(codeactions::code_actions(vault, &params, &path))
        })
        .await
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
    });
    Server::new(stdin, stdout, socket).serve(service).await; }
