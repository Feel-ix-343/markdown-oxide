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

        let matches = fuzzy_match_completions(
            &grep_match_text,
            completables,
            &self.link_completer.settings().case_matching,
        );

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
        let matches = fuzzy_match_completions(
            &filter_text,
            completables,
            &self.link_completer.settings().case_matching,
        );

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
            .into_par_iter()
            .find_any(|referenceable| match referenceable {
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
            _ => {
                let doc = Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: (block.range.start.line as isize - 1
                        ..=block.range.start.line as isize + 1)
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
                }));

                let should_index = completer.link_completer.settings().index_unindexed_blocks
                    && !completer.link_completer.settings().link_filenames_only;

                let (command, refname) = if should_index {
                    (
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
                        format!("{}#^{}", path_ref, rand_id),
                    )
                } else {
                    (None, path_ref)
                };

                (doc, command, CompletionItemKind::TEXT, None, refname)
            }
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
    ) -> Option<CompletionItem> {
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
    }
}

impl<'a> Completable<'a, UnindexedBlockCompleter<'a, WikiLinkCompleter<'a>>>
    for UnindexedBlock<'a>
{
    fn completions(
        &self,
        completer: &UnindexedBlockCompleter<'a, WikiLinkCompleter<'a>>,
    ) -> Option<CompletionItem> {
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
    }
}

impl Matchable for UnindexedBlock<'_> {
    fn match_string(&self) -> &str {
        self.0.text
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use crate::vault::{MyRange, Vault};
    use std::fs;
    use tower_lsp::lsp_types::ClientCapabilities;

    #[test]
    fn test_unindexed_block_link_filenames_only_omits_index_and_command() {
        let unique_dir = format!("test_moxide_unindexed_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let root = std::env::temp_dir().join(unique_dir);
        fs::create_dir_all(&root).unwrap();
        let test_file = root.join("note.md");
        fs::write(&test_file, "This is paragraph text without any index\n").unwrap();

        let settings = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        let vault = Vault::construct_vault(&settings, &root).unwrap();

        let link_completer = MarkdownLinkCompleter {
            path: ("note".to_string(), 0..4),
            display: ("".to_string(), 0..0),
            infile_ref: None,
            full_range: 0..0,
            line_nr: 0,
            position: Position { line: 0, character: 0 },
            vault: &vault,
            context_path: &test_file,
            settings: &settings,
        };

        let completer = UnindexedBlockCompleter {
            link_completer,
            new_id: "test1234".to_string(),
            __phantom: std::marker::PhantomData,
        };

        let block = Block {
            file: &test_file,
            range: MyRange(tower_lsp::lsp_types::Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 40 },
            }),
            text: "This is paragraph text without any index",
        };
        let unindexed = UnindexedBlock(block);

        // 1. Default settings: command is Some(apply_edits), refname contains ^test1234
        let (refname_default, item_default) = unindexed.partial_completion(&completer).unwrap();
        assert!(item_default.command.is_some());
        assert!(refname_default.contains("^test1234"));

        // 2. When link_filenames_only is true: command is None, refname is just the file name
        let mut settings_link_only = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        settings_link_only.link_filenames_only = true;
        let link_completer_link_only = MarkdownLinkCompleter {
            path: ("note".to_string(), 0..4),
            display: ("".to_string(), 0..0),
            infile_ref: None,
            full_range: 0..0,
            line_nr: 0,
            position: Position { line: 0, character: 0 },
            vault: &vault,
            context_path: &test_file,
            settings: &settings_link_only,
        };
        let completer_link_only = UnindexedBlockCompleter {
            link_completer: link_completer_link_only,
            new_id: "test1234".to_string(),
            __phantom: std::marker::PhantomData,
        };
        let (refname_link_only, item_link_only) = unindexed.partial_completion(&completer_link_only).unwrap();
        assert!(item_link_only.command.is_none());
        assert_eq!(refname_link_only, "note");

        // 3. When index_unindexed_blocks is false: command is None, refname is just the file name
        let mut settings_no_index = Settings::new(&root, &ClientCapabilities::default()).unwrap();
        settings_no_index.index_unindexed_blocks = false;
        let link_completer_no_index = MarkdownLinkCompleter {
            path: ("note".to_string(), 0..4),
            display: ("".to_string(), 0..0),
            infile_ref: None,
            full_range: 0..0,
            line_nr: 0,
            position: Position { line: 0, character: 0 },
            vault: &vault,
            context_path: &test_file,
            settings: &settings_no_index,
        };
        let completer_no_index = UnindexedBlockCompleter {
            link_completer: link_completer_no_index,
            new_id: "test1234".to_string(),
            __phantom: std::marker::PhantomData,
        };
        let (refname_no_index, item_no_index) = unindexed.partial_completion(&completer_no_index).unwrap();
        assert!(item_no_index.command.is_none());
        assert_eq!(refname_no_index, "note");

        let _ = fs::remove_dir_all(&root);
    }
}
