use itertools::Itertools;
use rayon::prelude::*;
use tower_lsp::lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionItemLabelDetails, Documentation,
    InsertTextFormat, MarkupContent, MarkupKind, Position, Range, TextEdit, Url,
};

use crate::{
    ui::preview_referenceable,
    vault::{get_obsidian_ref_path, Block, Referenceable},
};
use nanoid::nanoid;

use super::{
    link_completer::{LinkCompleter, MarkdownLinkCompleter, WikiLinkCompleter},
    matcher::{fuzzy_match_completions, Matchable},
    Completable, Completer,
};

pub struct UnindexedBlockCompleter<'a, T: LinkCompleter<'a>> {
    link_completer: T,
    new_id: String,
    __phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, C: LinkCompleter<'a>> UnindexedBlockCompleter<'a, C> {
    fn from_link_completer(link_completer: C) -> Option<UnindexedBlockCompleter<'a, C>> {
        if link_completer.entered_refname().starts_with(' ') {
            Some(UnindexedBlockCompleter::new(link_completer))
        } else {
            None
        }
    }

    fn new(completer: C) -> Self {
        let rand_id = nanoid!(
            5,
            &['a', 'b', 'c', 'd', 'e', 'f', 'g', '1', '2', '3', '4', '5', '6', '7', '8', '9']
        );

        Self {
            link_completer: completer,
            new_id: rand_id,
            __phantom: std::marker::PhantomData,
        }
    }

    fn completables(&self) -> Vec<UnindexedBlock<'a>> {
        let blocks = self.link_completer.vault().select_blocks();
        let position = self.link_completer.position();

        blocks
            .into_par_iter()
            .filter(|block| {
                !(block.range.start.line <= position.line
                    && block.range.start.character <= position.character
                    && block.range.end.line >= position.line
                    && block.range.end.character >= position.character)
            })
            .map(UnindexedBlock)
            .collect::<Vec<_>>()
    }

    fn grep_match_text(&self) -> String {
        self.link_completer.entered_refname()
    }
}

impl<'a> Completer<'a> for UnindexedBlockCompleter<'a, MarkdownLinkCompleter<'a>> {
    fn construct(context: super::Context<'a>, line: usize, character: usize) -> Option<Self>
    where
        Self: Sized,
    {
        let markdown_link_completer = MarkdownLinkCompleter::construct(context, line, character)?;

        Self::from_link_completer(markdown_link_completer)
    }

    fn completions(
        &self,
    ) -> Vec<impl super::Completable<'a, UnindexedBlockCompleter<'a, MarkdownLinkCompleter<'a>>>>
    where
        Self: Sized,
    {
        let completables = self.completables();

        let grep_match_text = self.grep_match_text();

        let matches = fuzzy_match_completions(&grep_match_text, completables);

        matches
    }

    type FilterParams = <MarkdownLinkCompleter<'a> as Completer<'a>>::FilterParams;
    fn completion_filter_text(&self, params: Self::FilterParams) -> String {
        self.link_completer.completion_filter_text(params)
    }
}

impl<'a> Completer<'a> for UnindexedBlockCompleter<'a, WikiLinkCompleter<'a>> {
    fn construct(context: super::Context<'a>, line: usize, character: usize) -> Option<Self>
    where
        Self: Sized,
    {
        let wiki_link_completer = WikiLinkCompleter::construct(context, line, character)?;

        UnindexedBlockCompleter::from_link_completer(wiki_link_completer)
    }

    fn completions(&self) -> Vec<impl Completable<'a, Self>>
    where
        Self: Sized,
    {
        let completables = self.completables();
        let filter_text = self.grep_match_text();
        let matches = fuzzy_match_completions(&filter_text, completables);

        matches
    }

    type FilterParams = <WikiLinkCompleter<'a> as Completer<'a>>::FilterParams;
    fn completion_filter_text(&self, params: Self::FilterParams) -> String {
        self.link_completer.completion_filter_text(params)
    }
}

struct UnindexedBlock<'a>(Block<'a>);

impl<'a> UnindexedBlock<'a> {
    /// Return the refname and completion item
    fn partial_completion<T: LinkCompleter<'a>>(
        &self,
        completer: &'a UnindexedBlockCompleter<'a, T>,
    ) -> Option<(String, CompletionItem)> {
        let rand_id = &completer.new_id;

        let path_ref =
            get_obsidian_ref_path(completer.link_completer.vault().root_dir(), self.0.file)?;
        let url = Url::from_file_path(self.0.file).ok()?;

        let block = self.0;

        // check if the block is already indexed
        let (documentation, command, kind, label_detail, refname): (
            Option<Documentation>,
            Option<Command>,
            CompletionItemKind,
            Option<CompletionItemLabelDetails>,
            String,
        ) = match completer
            .link_completer
            .vault()
            .select_referenceable_nodes(Some(block.file))
            .into_iter()
            .find(|referenceable| match referenceable {
                Referenceable::IndexedBlock(_path, indexed_block) => {
                    indexed_block.range.start.line == block.range.start.line
                }
                _ => false,
            }) {
            Some(ref referenceable @ Referenceable::IndexedBlock(_, indexed_block)) => (
                preview_referenceable(completer.link_completer.vault(), referenceable)
                    .map(Documentation::MarkupContent),
                None,
                CompletionItemKind::REFERENCE,
                Some(CompletionItemLabelDetails {
                    detail: Some("Indexed Block".to_string()),
                    description: None,
                }),
                format!("{}#^{}", path_ref, indexed_block.index),
            ),
            _ => (
                Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: (block.range.start.line as isize - 5
                        ..=block.range.start.line as isize + 5)
                        .flat_map(|i| {
                            Some((
                                completer
                                    .link_completer
                                    .vault()
                                    .select_line(block.file, i)?,
                                i,
                            ))
                        })
                        .map(|(iter, ln)| {
                            if ln == block.range.start.line as isize {
                                format!("**{}**\n", String::from_iter(iter).trim())
                                // highlight the block to be references
                            } else {
                                String::from_iter(iter)
                            }
                        })
                        .join(""),
                })),
                Some(Command {
                    title: "Insert Block Reference Into File".into(),
                    command: "apply_edits".into(),
                    arguments: Some(vec![serde_json::to_value(
                        tower_lsp::lsp_types::WorkspaceEdit {
                            changes: Some(
                                vec![(
                                    url,
                                    vec![TextEdit {
                                        range: Range {
                                            start: Position {
                                                line: block.range.end.line,
                                                character: block.range.end.character - 1,
                                            },
                                            end: Position {
                                                line: block.range.end.line,
                                                character: block.range.end.character - 1,
                                            },
                                        },
                                        new_text: format!("   ^{}", rand_id),
                                    }],
                                )]
                                .into_iter()
                                .collect(),
                            ),
                            change_annotations: None,
                            document_changes: None,
                        },
                    )
                    .ok()?]),
                }),
                CompletionItemKind::TEXT,
                None,
                format!("{}#^{}", path_ref, rand_id),
            ),
        };

        Some((
            refname,
            CompletionItem {
                label: block.text.to_string(),
                documentation,
                // Insert the index for the block
                command,
                kind: Some(kind),
                label_details: label_detail,
                ..Default::default()
            },
        ))
    }
}

impl<'a> Completable<'a, UnindexedBlockCompleter<'a, MarkdownLinkCompleter<'a>>>
    for UnindexedBlock<'a>
{
    fn completions(
        &self,
        completer: &UnindexedBlockCompleter<'a, MarkdownLinkCompleter<'a>>,
    ) -> impl Iterator<Item = CompletionItem> {
        (|| {
            let (refname, partial_completion) = self.partial_completion(completer)?;

            let binding = completer.link_completer.entered_refname();
            let display = &binding.trim();

            Some(CompletionItem {
                text_edit: Some(
                    completer
                        .link_completer
                        .completion_text_edit(Some(&format!("${{1:{}}}", display)), &refname),
                ),
                filter_text: Some(
                    completer.completion_filter_text(&completer.link_completer.entered_refname()),
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..partial_completion
            })
        })()
        .into_iter()
    }
}

impl<'a> Completable<'a, UnindexedBlockCompleter<'a, WikiLinkCompleter<'a>>>
    for UnindexedBlock<'a>
{
    fn completions(
        &self,
        completer: &UnindexedBlockCompleter<'a, WikiLinkCompleter<'a>>,
    ) -> impl Iterator<Item = CompletionItem> {
        (|| {
            let (refname, partial_completion) = self.partial_completion(completer)?;

            let binding = completer.link_completer.entered_refname();
            let display = &binding.trim();

            Some(CompletionItem {
                text_edit: Some(
                    completer
                        .link_completer
                        .completion_text_edit(Some(&format!("${{1:{}}}", display)), &refname),
                ),
                filter_text: Some(
                    completer.completion_filter_text(&completer.link_completer.entered_refname()),
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..partial_completion
            })
        })()
        .into_iter()
    }
}

impl Matchable for UnindexedBlock<'_> {
    fn match_string(&self) -> &str {
        self.0.text
    }
}
