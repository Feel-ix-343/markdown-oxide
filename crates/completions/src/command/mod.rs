use actions::{Actions, AppendBlockIndex, UpsertEntityReference};
use tower_lsp::lsp_types::CompletionItemKind;

pub mod actions;

pub struct Command<A: Actions> {
    label: String,
    kind: CompletionItemKind,
    /// Displayed in a preview beside the command as it is being selected
    cmd_ui_info: String,
    actions: A,
}

pub type LinkBlockCmd<'a> = Command<(UpsertEntityReference<'a>, AppendBlockIndex<'a>)>;
pub type ReferenceNamedSectionCmd<'a> = Command<UpsertEntityReference<'a>>;
