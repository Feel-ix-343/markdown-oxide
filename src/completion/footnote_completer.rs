use std::path::Path;

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation};

use crate::{ui::preview_referenceable, vault::{MDFootnote, Preview, Referenceable, Vault}};

use super::{Completable, Completer};

use rayon::prelude::*;

pub struct FootnoteCompleter<'a> {
    vault: &'a Vault,
    line: usize,
    character: usize,
    path: &'a Path
}

impl<'a> Completer<'a> for FootnoteCompleter<'a> {
    fn construct(context: super::Context<'a>, line: usize, character: usize) -> Option<Self>
        where Self: Sized + Completer<'a> {
        

        let selected_line = context.vault.select_line(context.path, line as isize)?;

        if character
            .checked_sub(1)
            .and_then(|start| selected_line.get(start..character))
        == Some(&['[']) {
            Some(FootnoteCompleter{path: context.path, character, line, vault: context.vault})
        } else {
            None
        }
    }

    fn completions(&self) -> Vec<impl super::Completable<'a, Self>> where Self: Sized {
        
        let path_footnotes = self.vault.select_referenceable_nodes(Some(&self.path))
            .into_par_iter()
            .flat_map(|referenceable| FootnoteCompletion::from_referenceable(referenceable))
            .collect::<Vec<_>>();

        path_footnotes


    }

    type FilterParams = (&'a str, Referenceable<'a>);
    fn completion_filter_text(&self, params: Self::FilterParams) -> String {
        self.vault
            .select_referenceable_preview(&params.1)
            .and_then(|preview| match preview {
                Preview::Text(string) => Some(string),
                Preview::Empty => None,
            })
            .map(|preview_string| format!("{}{}", params.0, &preview_string)).unwrap_or("".to_owned())
    }

}

struct FootnoteCompletion<'a> {
    footnote: (&'a Path, &'a MDFootnote)
}

    impl FootnoteCompletion<'_> {
        fn from_referenceable(referenceable: Referenceable<'_>) -> Option<FootnoteCompletion<'_>> {
            match referenceable {
                Referenceable::Footnote(path, footnote) => Some(FootnoteCompletion{footnote: (path, footnote)}),
                _ => None
            }
        }
    }


impl<'a> Completable<'a, FootnoteCompleter<'a>> for FootnoteCompletion<'a> {
    fn completions(&self, completer: &FootnoteCompleter<'a>) -> impl Iterator<Item = tower_lsp::lsp_types::CompletionItem> {




        let refname = &self.footnote.1.index;

        let path = self.footnote.0;
        let path_buf = path.to_path_buf();
        let self_referenceable = Referenceable::Footnote(&path_buf, self.footnote.1);

        Some(CompletionItem {
            label: refname.to_string(),
            kind: Some(CompletionItemKind::REFERENCE),
            documentation: preview_referenceable(completer.vault, &self_referenceable)
                .map(Documentation::MarkupContent),
            filter_text: Some(completer.completion_filter_text((refname, self_referenceable))),
            ..Default::default()
        }).into_iter()
        
    }
}
