use rayon::prelude::*;
use do_notation::m;
use tower_lsp::lsp_types::{CompletionItem, CompletionTextEdit, Documentation, MarkupContent, MarkupKind};



pub(super) trait Completer{
    fn completions(&self, location: &Location) -> Option<impl IndexedParallelIterator<Item = Box<dyn Completion>>>;
}

pub(super) trait Completion{
    fn label(&self) -> String;
    fn label_detail(&self) -> Option<String>;
    fn kind(&self) -> tower_lsp::lsp_types::CompletionItemKind;
    fn detail(&self) -> Option<String>;
    fn documentation(&self) -> Option<String>;
    fn deprecated(&self) -> Option<bool>;
    fn preselect(&self) -> Option<bool>;
    fn filter_text(&self) -> Option<String>;
    fn text_edit(&self) -> tower_lsp::lsp_types::TextEdit;
    fn additional_text_edits(&self) -> Option<Vec<tower_lsp::lsp_types::TextEdit>>;
    fn command(&self) -> Option<tower_lsp::lsp_types::Command>;
    fn commit_characters(&self) -> Option<Vec<String>>;
    
    // TODO: possibly an ID to handle completion resolve
}

pub (super) struct Location;

pub (super) trait Context {
    fn max_query_completion_items(&self) -> usize;
}

pub (super) fn completions(
    location: &Location,
    context: &impl Context,
    referencer: &impl Completer,
    syntax_completer: &impl Completer,
    actions_completer: &impl Completer
) -> Option<tower_lsp::lsp_types::CompletionResponse> {

    let actions_completions = actions_completer.completions(location);
    let syntax_completions = syntax_completer.completions(location);
    let references_completions = referencer.completions(location);

    let all_completions = m! {
        actions <- actions_completions;
        syntax_completions <- syntax_completions;
        references_completions <- references_completions;

        Some(
            actions
                .chain(syntax_completions)
                .chain(references_completions.take(context.max_query_completion_items()))
        )
    }?;


    Some(
        tower_lsp::lsp_types::CompletionResponse::List(
            tower_lsp::lsp_types::CompletionList {
                is_incomplete: true,
                items: completion_items(all_completions)
            }
        )
    )
}

fn completion_items(completion_items: impl IndexedParallelIterator<Item = Box<dyn Completion>>) 
    -> Vec<tower_lsp::lsp_types::CompletionItem> {

    completion_items.into_par_iter()
        .enumerate()
        .map(|(idx, completion)| CompletionItem {
            label: completion.label(),
            kind: Some(completion.kind()),
            detail: completion.detail(),
            documentation: m! {
                documentation <- completion.documentation();

                Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: documentation
                }))
            },
            deprecated: completion.deprecated(),
            preselect: completion.preselect(),
            filter_text: completion.filter_text(),
            text_edit: Some(CompletionTextEdit::Edit(completion.text_edit())),
            additional_text_edits: completion.additional_text_edits(),
            command: completion.command(),
            commit_characters: completion.commit_characters(),
            sort_text: Some(idx.to_string()),
            ..Default::default()
        })
        .collect()

}
