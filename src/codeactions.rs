use std::path::{Path, PathBuf};

use pathdiff::diff_paths;
use rayon::iter::*;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionOrCommand, CodeActionParams, CreateFile, DocumentChangeOperation,
    DocumentChanges, ResourceOp, Url, WorkspaceEdit,
};

use crate::{
    diagnostics::path_unresolved_references,
    vault::{Reference, Vault},
};

pub fn code_actions(
    vault: &Vault,
    params: &CodeActionParams,
    path: &Path,
) -> Option<Vec<CodeActionOrCommand>> {
    // Diagnostics
    // get all links for changed file

    let unresolved = path_unresolved_references(vault, path)?;

    let unresolved_file_links = unresolved
        .into_iter()
        .filter(|(_, reference)| matches!(reference, Reference::FileLink(..)));

    let code_action_unresolved = unresolved_file_links.filter(|(_, reference)| {
        reference.data().range.start.line <= params.range.start.line
            && reference.data().range.end.line >= params.range.end.line
            && reference.data().range.start.character <= params.range.start.character
            && reference.data().range.end.character >= params.range.end.character
    });

    Some(
        code_action_unresolved
            .filter_map(|(_path, reference)| {
                let mut new_path_buf = PathBuf::new();
                new_path_buf.push(vault.root_dir());
                new_path_buf.push(&reference.data().reference_text);
                new_path_buf.set_extension("md");

                let new_path = Url::from_file_path(&new_path_buf).ok()?;

                Some(CodeActionOrCommand::CodeAction(CodeAction {
                    title: format!(
                        "Create File: {:?}",
                        diff_paths(new_path_buf, vault.root_dir())?
                    ),
                    edit: Some(WorkspaceEdit {
                        document_changes: Some(DocumentChanges::Operations(vec![
                            DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                                uri: new_path,
                                options: None,
                                annotation_id: None,
                            })),
                        ])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }))
            })
            .collect(),
    )
}
