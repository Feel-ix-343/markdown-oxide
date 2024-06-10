use rayon::prelude::*;
use do_notation::m;
use tower_lsp::lsp_types::{CompletionItem, CompletionTextEdit, Documentation, MarkupContent, MarkupKind};

use crate::context::Context;

pub(super) trait Completer {
    fn completions(&self, cx: &Context, location: &Location) -> Option<Vec<Box<dyn Completion>>>;
}

pub(super) trait Completion: Send + Sync  {
    fn label(&self, cx: &Context) -> String;
    fn label_detail(&self, cx: &Context) -> Option<String>;
    fn kind(&self, cx: &Context) -> tower_lsp::lsp_types::CompletionItemKind;
    fn detail(&self, cx: &Context) -> Option<String>;
    fn documentation(&self, cx: &Context) -> Option<String>;
    fn deprecated(&self, cx: &Context) -> Option<bool>;
    fn preselect(&self, cx: &Context) -> Option<bool>;
    fn text_edit(&self, cx: &Context) -> tower_lsp::lsp_types::TextEdit;
    fn command(&self, cx: &Context) -> Option<tower_lsp::lsp_types::Command>;
    
    // TODO: possibly an ID to handle completion resolve
}

pub (super) struct Location {
    pub(crate) line: u32,
    pub(crate) character: u32,
    pub(crate) file: String
}


pub (super) fn completions(
    location: &Location,
    cx: &Context,
    referencer: &impl Completer,
    // syntax_completer: &impl Completer,
    // actions_completer: &impl Completer
) -> Option<tower_lsp::lsp_types::CompletionResponse> {

    // let actions_completions = actions_completer.completions(location);
    // let syntax_completions = syntax_completer.completions(location);
    let references_completions = referencer.completions(cx, location);

    let all_completions = m! {
        // actions <- actions_completions;
        // syntax_completions <- syntax_completions;
        references_completions <- references_completions;

        Some(
            // actions
            //     .chain(syntax_completions)
            //     .chain(references_completions.take(context.max_query_completion_items()))
            references_completions
        )
    }?;


    Some(
        tower_lsp::lsp_types::CompletionResponse::List(
            tower_lsp::lsp_types::CompletionList {
                is_incomplete: true,
                items: completion_items(all_completions, cx)
            }
        )
    )
}

fn completion_items(completion_items: Vec<Box<dyn Completion + '_>>, cx: &Context) 
    -> Vec<tower_lsp::lsp_types::CompletionItem> {

    completion_items.into_par_iter()
        .enumerate()
        .map(|(idx, completion)| CompletionItem {
            label: completion.label(cx),
            kind: Some(completion.kind(cx)),
            detail: completion.detail(cx),
            documentation: m! {
                documentation <- completion.documentation(cx);

                Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: documentation
                }))
            },
            deprecated: completion.deprecated(cx),
            preselect: completion.preselect(cx),
            filter_text: Some(cx.parser().entered_completion_string().to_string()),
            text_edit: Some(CompletionTextEdit::Edit(completion.text_edit(cx))),
            command: completion.command(cx),
            sort_text: Some(idx.to_string()),
            ..Default::default()
        })
        .collect()

}
