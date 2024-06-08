use rayon::prelude::*;
use do_notation::m;
use tower_lsp::lsp_types::{CompletionItem, CompletionTextEdit, Documentation, MarkupContent, MarkupKind};

use crate::adapters::{Context, Parser};

pub(super) trait Completer {
    fn completions(&self, location: &Location) -> Option<impl IndexedParallelIterator<Item = Box<dyn Completion>>>;
}

pub(super) trait Completion: Send + Sync  {
    fn label(&self) -> String;
    fn label_detail(&self) -> Option<String>;
    fn kind(&self) -> tower_lsp::lsp_types::CompletionItemKind;
    fn detail(&self) -> Option<String>;
    fn documentation(&self) -> Option<String>;
    fn deprecated(&self) -> Option<bool>;
    fn preselect(&self) -> Option<bool>;
    fn text_edit(&self) -> tower_lsp::lsp_types::TextEdit;
    fn command(&self) -> Option<tower_lsp::lsp_types::Command>;
    
    // TODO: possibly an ID to handle completion resolve
}

pub (super) struct Location;


pub (super) fn completions(
    location: &Location,
    context: &Context,
    parser: &Parser,
    referencer: &impl Completer,
    syntax_completer: &impl Completer,
    actions_completer: &impl Completer
) -> Option<tower_lsp::lsp_types::CompletionResponse> {

    // let actions_completions = actions_completer.completions(location);
    // let syntax_completions = syntax_completer.completions(location);
    let references_completions = referencer.completions(location);

    let all_completions = m! {
        // actions <- actions_completions;
        // syntax_completions <- syntax_completions;
        references_completions <- references_completions;

        Some(
            // actions
            //     .chain(syntax_completions)
            //     .chain(references_completions.take(context.max_query_completion_items()))
            references_completions.take(context.max_query_completion_items())
        )
    }?;


    Some(
        tower_lsp::lsp_types::CompletionResponse::List(
            tower_lsp::lsp_types::CompletionList {
                is_incomplete: true,
                items: completion_items(all_completions, parser)
            }
        )
    )
}

fn completion_items(completion_items: impl IndexedParallelIterator<Item = Box<dyn Completion>>, parser: &Parser) 
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
            filter_text: Some(parser.entered_completion_string().to_string()),
            text_edit: Some(CompletionTextEdit::Edit(completion.text_edit())),
            command: completion.command(),
            sort_text: Some(idx.to_string()),
            ..Default::default()
        })
        .collect()

}
