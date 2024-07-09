use actions::{Actions, AppendBlockIndex, UpsertEntityReference};
use tower_lsp::lsp_types::CompletionItemKind;

pub mod actions;

pub struct Command<A: Actions> {
    pub label: String,
    pub kind: CompletionItemKind,
    pub label_detail: Option<String>,

    /// Displayed in a preview beside the command as it is being selected
    pub cmd_ui_info: Option<String>,
    pub actions: A,
}

pub type LinkBlockCmd = Command<(UpsertEntityReference, Option<AppendBlockIndex>)>;
pub type ReferenceNamedSectionCmd = Command<UpsertEntityReference>;
