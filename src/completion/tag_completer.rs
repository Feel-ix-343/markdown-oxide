use std::path::Path;

use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit,
    Documentation, Position, Range, TextEdit,
};

use crate::{
    completion::util::check_in_code_block,
    ui,
    vault::{MDTag, Referenceable, Vault},
};

use super::{
    matcher::{fuzzy_match_completions, Matchable},
    Completable, Completer, Context, LineRange,
};

use rayon::prelude::*;

pub struct TagCompleter<'a> {
    full_range: LineRange<usize>,
    /// Tag name and range not including the '#'
    inputted_tag: (String, LineRange<usize>),
    vault: &'a Vault,
    line: usize,
    character: usize,
    context: Context<'a>,
}

impl<'a> Completer<'a> for TagCompleter<'a> {
    fn construct(context: super::Context<'a>, line: usize, character: usize) -> Option<Self>
    where
        Self: Sized + Completer<'a>,
    {
        if !context.settings.tags_in_codeblocks && check_in_code_block(&context, line, character) {
            return None;
        }

        static PARTIAL_TAG_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\#(?<text>[a-zA-Z0-9/_-]*)").unwrap());

        let line_chars = context.vault.select_line(context.path, line as isize)?;
        let line_string = String::from_iter(line_chars);

        let captures_iter = PARTIAL_TAG_REGEX.captures_iter(&line_string);

        captures_iter
            .flat_map(|captures| {
                let (full, tag_text) = (captures.get(0)?, captures.name("text")?);

                // check if the cursor is in the tag
                let preceding_character = character - 1; // User is inserting into the position after the character they are looking at; "#tag|"  cursor is a position 4; I want pos 3; the end of the tag
                if preceding_character >= full.range().start
                    && preceding_character < full.range().end
                {
                    // end is exclusive
                    Some(TagCompleter {
                        full_range: full.range(),
                        inputted_tag: (tag_text.as_str().to_string(), tag_text.range()),
                        vault: context.vault,
                        line,
                        character,
                        context,
                    })
                } else {
                    None
                }
            })
            .next()
    }

    fn completions(&self) -> Vec<impl super::Completable<'a, Self>>
    where
        Self: Sized,
    {
        let tag_referenceables = self
            .vault
            .select_referenceable_nodes(None)
            .into_par_iter()
            .flat_map(TagCompletable::from_referenceable)
            .filter(|tag| {
                !(tag.tag.1.range.start.line <= self.line as u32
                    && tag.tag.1.range.start.character <= self.character as u32
                    && tag.tag.1.range.end.line >= self.line as u32
                    && tag.tag.1.range.end.character >= self.character as u32)
            })
            .collect::<Vec<_>>();

        // uniqued
        let tag_referenceables = tag_referenceables
            .into_iter()
            .unique_by(|tag| tag.match_string().to_owned())
            .collect::<Vec<_>>();

        let filter_text = &self.inputted_tag.0;

        let filtered = fuzzy_match_completions(
            filter_text,
            tag_referenceables,
            &self.context.settings.case_matching,
        );

        filtered
    }

    type FilterParams = &'a str;

    fn completion_filter_text(&self, params: Self::FilterParams) -> String {
        format!("#{}", params)
    }
}

struct TagCompletable<'a> {
    tag: (&'a Path, &'a MDTag),
}

impl TagCompletable<'_> {
    fn from_referenceable(referenceable: Referenceable<'_>) -> Option<TagCompletable<'_>> {
        match referenceable {
            Referenceable::Tag(path, tag) => Some(TagCompletable { tag: (path, tag) }),
            _ => None,
        }
    }
}

impl Matchable for TagCompletable<'_> {
    fn match_string(&self) -> &str {
        &self.tag.1.tag_ref
    }
}

impl<'a> Completable<'a, TagCompleter<'a>> for TagCompletable<'a> {
    fn completions(&self, completer: &TagCompleter<'a>) -> Option<CompletionItem> {
        let text_edit = CompletionTextEdit::Edit(TextEdit {
            new_text: format!("#{}", self.tag.1.tag_ref),
            range: Range {
                start: Position {
                    line: completer.line as u32,
                    character: completer.full_range.start as u32,
                },
                end: Position {
                    line: completer.line as u32,
                    character: completer.full_range.end as u32,
                },
            },
        });

        let path = self.tag.0;
        let path_buf = path.to_path_buf();
        let self_as_referenceable = Referenceable::Tag(&path_buf, self.tag.1);

        let num_references = completer
            .vault
            .select_references_for_referenceable(&self_as_referenceable)
            .map(|references| references.len())
            .unwrap_or(0);

        Some(CompletionItem {
            label: self.tag.1.tag_ref.clone(),
            kind: Some(CompletionItemKind::KEYWORD),
            filter_text: Some(completer.completion_filter_text(&self.tag.1.tag_ref.clone())),
            documentation: ui::preview_referenceable(completer.vault, &self_as_referenceable)
                .map(Documentation::MarkupContent),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(match num_references {
                    1 => "1 reference".to_string(),
                    n => format!("{} references", n),
                }),
                description: None,
            }),
            text_edit: Some(text_edit),
            ..Default::default()
        })
    }
}
