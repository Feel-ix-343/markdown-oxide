use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::{CompletionItem, CompletionList, CompletionParams, CompletionResponse};

use crate::{config::Settings, vault::Vault};

use self::callout_completer::CalloutCompleter;
use self::link_completer::WikiLinkCompleter;
use self::{
    footnote_completer::FootnoteCompleter, link_completer::MarkdownLinkCompleter,
    tag_completer::TagCompleter, unindexed_block_completer::UnindexedBlockCompleter,
};

mod callout_completer;
mod footnote_completer;
mod link_completer;
mod matcher;
mod tag_completer;
mod unindexed_block_completer;
mod util;

#[derive(Clone, Copy)]
pub struct Context<'a> {
    vault: &'a Vault,
    opened_files: &'a [PathBuf],
    path: &'a Path,
    settings: &'a Settings,
}

pub trait Completer<'a>: Sized {
    fn construct(context: Context<'a>, line: usize, character: usize) -> Option<Self>
    where
        Self: Sized + Completer<'a>;

    fn completions(&self) -> Vec<impl Completable<'a, Self>>
    where
        Self: Sized;

    type FilterParams;
    /// Completere like nvim-cmp are odd so manually define the filter text as a situational workaround
    fn completion_filter_text(&self, params: Self::FilterParams) -> String;

    // fn compeltion_resolve(&self, vault: &Vault, resolve_item: CompletionItem) -> Option<CompletionItem>;
}

pub trait Completable<'a, T: Completer<'a>>: Sized {
    fn completions(&self, completer: &T) -> Option<CompletionItem>;
}

/// Range indexes for one line of the file; NOT THE WHOLE FILE
type LineRange<T> = std::ops::Range<T>;

pub fn get_completions(
    vault: &Vault,
    initial_completion_files: &[PathBuf],
    params: &CompletionParams,
    path: &Path,
    config: &Settings,
) -> Option<CompletionResponse> {
    let completion_context = Context {
        vault,
        opened_files: initial_completion_files,
        path,
        settings: config,
    };

    // I would refactor this if I could figure out generic closures
    run_completer::<UnindexedBlockCompleter<MarkdownLinkCompleter>>(
        completion_context,
        params.text_document_position.position.line,
        params.text_document_position.position.character,
    )
    .or_else(|| {
        run_completer::<UnindexedBlockCompleter<WikiLinkCompleter>>(
            completion_context,
            params.text_document_position.position.line,
            params.text_document_position.position.character,
        )
    })
    .or_else(|| {
        run_completer::<MarkdownLinkCompleter>(
            completion_context,
            params.text_document_position.position.line,
            params.text_document_position.position.character,
        )
    })
    .or_else(|| {
        run_completer::<WikiLinkCompleter>(
            completion_context,
            params.text_document_position.position.line,
            params.text_document_position.position.character,
        )
    })
    .or_else(|| {
        run_completer::<TagCompleter>(
            completion_context,
            params.text_document_position.position.line,
            params.text_document_position.position.character,
        )
    })
    .or_else(|| {
        run_completer::<FootnoteCompleter>(
            completion_context,
            params.text_document_position.position.line,
            params.text_document_position.position.character,
        )
    })
    .or_else(|| {
        run_completer::<CalloutCompleter>(
            completion_context,
            params.text_document_position.position.line,
            params.text_document_position.position.character,
        )
    })
}

// #[cfg(test)]
// mod tests {
//     use itertools::Itertools;
//
//     use super::{get_wikilink_index, CompletableMDLink, CompletableTag, get_completable_tag};
//
//     #[test]
//     fn test_index() {
//         let s = "test [[linjfkdfjds]]";
//
//         let expected = 6;
//
//         let actual = get_wikilink_index(&s.chars().collect(), 10);
//
//         assert_eq!(Some(expected), actual);
//
//         assert_eq!(Some("lin"), s.get(expected + 1..10));
//     }
//
//     #[test]
//     fn test_partial_mdlink() {
//         let line = "This is line [display](partialpa"; // (th)
//
//         let expected = Some(CompletableMDLink {
//             partial: ("[display](partialpa".to_string(), 13..32),
//             display: ("display".to_string(), 14..21),
//             path: ("partialpa".to_string(), 23..32),
//             infile_ref: None,
//             full_range: 13..32,
//         });
//
//         let actual = super::get_completable_mdlink(&line.chars().collect(), 32);
//
//         assert_eq!(actual, expected);
//
//         let line = "This is line [display](partialpath)"; // (th)
//
//         let expected = Some(CompletableMDLink {
//             partial: ("[display](partialpa".to_string(), 13..32),
//             display: ("display".to_string(), 14..21),
//             path: ("partialpa".to_string(), 23..32),
//             infile_ref: None,
//             full_range: 13..35,
//         });
//
//         let actual = super::get_completable_mdlink(&line.chars().collect(), 32);
//
//         assert_eq!(actual, expected);
//
//         let line = "[disp](pp) This is line [display](partialpath)"; // (th)
//
//         let expected = Some(CompletableMDLink {
//             partial: ("[display](partialpa".to_string(), 24..43),
//             display: ("display".to_string(), 25..32),
//             path: ("partialpa".to_string(), 34..43),
//             infile_ref: None,
//             full_range: 24..46,
//         });
//
//         let actual = super::get_completable_mdlink(&line.chars().collect(), 43);
//
//         assert_eq!(actual, expected);
//
//         let line = "[disp](pp) This is line [display](partialpath)"; // (th)
//
//         let expected = Some(CompletableMDLink {
//             partial: ("[display](partialpath".to_string(), 24..45),
//             display: ("display".to_string(), 25..32),
//             path: ("partialpath".to_string(), 34..45),
//             infile_ref: None,
//             full_range: 24..46,
//         });
//
//         let actual = super::get_completable_mdlink(&line.chars().collect(), 45);
//
//         assert_eq!(actual, expected);
//     }
//
//     #[test]
//     fn test_partial_mdlink_infile_refs() {
//         let line = "This is line [display](partialpa#"; // (th)
//
//         let expected = Some(CompletableMDLink {
//             partial: ("[display](partialpa#".to_string(), 13..33),
//             display: ("display".to_string(), 14..21),
//             path: ("partialpa".to_string(), 23..32),
//             infile_ref: Some(("".to_string(), 33..33)),
//             full_range: 13..33,
//         });
//
//         let actual = super::get_completable_mdlink(&line.chars().collect(), 33);
//
//         assert_eq!(actual, expected);
//
//         let line = "[disp](pp) This is line [display](partialpath#Display)"; // (th)
//
//         let expected = Some(CompletableMDLink {
//             partial: ("[display](partialpath#Display".to_string(), 24..53),
//             display: ("display".to_string(), 25..32),
//             path: ("partialpath".to_string(), 34..45),
//             infile_ref: Some(("Display".to_string(), 46..53)),
//             full_range: 24..54,
//         });
//
//         let actual = super::get_completable_mdlink(&line.chars().collect(), 53);
//
//         assert_eq!(actual, expected);
//
//         let line = "[disp](pp) This is line [display](partialpath#Display)"; // (th)
//
//         let expected = Some(CompletableMDLink {
//             partial: ("[display](partialpath#Disp".to_string(), 24..50),
//             display: ("display".to_string(), 25..32),
//             path: ("partialpath".to_string(), 34..45),
//             infile_ref: Some(("Disp".to_string(), 46..50)),
//             full_range: 24..54,
//         });
//
//         let actual = super::get_completable_mdlink(&line.chars().collect(), 50);
//
//         assert_eq!(actual, expected);
//     }
//
//     #[test]
//     fn test_completable_tag_parsing() {
//         //          0         1         2
//         //          01234567890123456789012345678
//         let text = "text over here #tag more text";
//
//         let insert_position = 19;
//
//         let expected = CompletableTag {
//             full_range: 15..19,
//             inputted_tag: ("tag".to_string(), 16..19) // not inclusive
//         };
//
//         let actual = get_completable_tag(&text.chars().collect_vec(), insert_position);
//
//
//         assert_eq!(Some(expected), actual);
//
//
//
//         //          0         1         2
//         //          01234567890123456789012345678
//         let text = "text over here #tag more text";
//
//         let insert_position = 20;
//
//         let actual = get_completable_tag(&text.chars().collect_vec(), insert_position);
//
//
//         assert_eq!(None, actual);
//
//
//         //          0         1         2
//         //          01234567890123456789012345678
//         let text = "text over here # more text";
//
//         let insert_position = 16;
//
//         let actual = get_completable_tag(&text.chars().collect_vec(), insert_position);
//
//         let expected = Some(CompletableTag {
//             full_range: 15..16,
//             inputted_tag: ("".to_string(), 16..16)
//         });
//
//
//         assert_eq!(expected, actual);
//
//
//         //          0         1         2
//         //          01234567890123456789012345678
//         let text = "text over here #tag mor #tag ";
//
//         let insert_position = 28;
//
//         let expected = CompletableTag {
//             full_range: 24..28,
//             inputted_tag: ("tag".to_string(), 25..28) // not inclusive
//         };
//
//         let actual = get_completable_tag(&text.chars().collect_vec(), insert_position);
//
//
//         assert_eq!(Some(expected), actual);
//
//
//     }
// }

fn run_completer<'a, T: Completer<'a>>(
    context: Context<'a>,
    line: u32,
    character: u32,
) -> Option<CompletionResponse> {
    let completer = T::construct(context, line as usize, character as usize)?;
    let completions = completer.completions();

    let completions = completions
        .into_iter()
        .take(20)
        .flat_map(|completable| {
            completable
                .completions(&completer)
                .into_iter()
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect::<Vec<CompletionItem>>();

    // Return None instead of an empty completion list. An empty list with
    // is_incomplete: true can cause editors (e.g. Zed) to keep a completion
    // session alive, which may interfere with normal editing behaviour such
    // as auto-indentation in list contexts (see #334). Returning None lets
    // the editor fall back to its default behaviour and also allows the
    // completer chain in get_completions to try the next completer.
    if completions.is_empty() {
        return None;
    }

    Some(CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items: completions,
    }))
}

#[cfg(test)]
mod completion_tests {
    use std::path::PathBuf;

    use tower_lsp::lsp_types::{
        CompletionContext, CompletionParams, CompletionTriggerKind, PartialResultParams, Position,
        TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams,
    };

    use crate::config::Settings;
    use crate::vault::Vault;

    use super::get_completions;

    fn test_vault_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("TestFiles")
    }

    fn test_settings() -> Settings {
        let root = test_vault_root();
        let caps = Default::default();
        Settings::new(&root, &caps).expect("Settings should load from TestFiles")
    }

    fn make_completion_params(uri: Url, line: u32, character: u32) -> CompletionParams {
        CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        }
    }

    /// Regression test for #334: typing in a numbered list with a nested list
    /// should not produce completions that could interfere with editor
    /// auto-indentation.
    #[test]
    fn no_completions_for_numbered_list_content() {
        let root = test_vault_root();
        let settings = test_settings();
        let vault = Vault::construct_vault(&settings, &root).expect("Vault should construct");

        // Simulate a file with the MRE content from #334
        let test_path = root.join("_list_test.md");
        let test_text = "1. a\n  * a\n  * b\n2";

        let mut vault = vault;
        Vault::update_vault(&settings, &mut vault, (&test_path, test_text));

        let uri = Url::from_file_path(&test_path).unwrap();
        let opened_files: Vec<PathBuf> = vec![test_path.clone()];

        // Cursor at end of line 3 (after typing "2")
        let params = make_completion_params(uri.clone(), 3, 1);
        let result = get_completions(&vault, &opened_files, &params, &test_path, &settings);
        assert!(
            result.is_none(),
            "No completions should be returned when typing in a numbered list item"
        );

        // Also test cursor at the beginning of an empty line (line 3 before typing)
        let test_text_empty_line = "1. a\n  * a\n  * b\n";
        Vault::update_vault(&settings, &mut vault, (&test_path, test_text_empty_line));
        let params = make_completion_params(uri.clone(), 3, 0);
        let result = get_completions(&vault, &opened_files, &params, &test_path, &settings);
        assert!(
            result.is_none(),
            "No completions should be returned on an empty line in a list"
        );

        // Test with unordered top-level list (should also return no completions)
        let test_text_unordered = "* a\n  * a\n  * b\n2";
        Vault::update_vault(&settings, &mut vault, (&test_path, test_text_unordered));
        let params = make_completion_params(uri, 3, 1);
        let result = get_completions(&vault, &opened_files, &params, &test_path, &settings);
        assert!(
            result.is_none(),
            "No completions should be returned when typing in an unordered list"
        );
    }

    /// Verify that empty completion lists are not returned (they become None).
    #[test]
    fn empty_completions_return_none() {
        let root = test_vault_root();
        let settings = test_settings();
        let vault = Vault::construct_vault(&settings, &root).expect("Vault should construct");

        let test_path = root.join("_empty_test.md");
        let test_text = "Just some plain text with no links or tags.";

        let mut vault = vault;
        Vault::update_vault(&settings, &mut vault, (&test_path, test_text));

        let uri = Url::from_file_path(&test_path).unwrap();
        let opened_files: Vec<PathBuf> = vec![test_path.clone()];

        // Typing in plain text should not produce completions
        let params = make_completion_params(uri, 0, 10);
        let result = get_completions(&vault, &opened_files, &params, &test_path, &settings);
        assert!(
            result.is_none(),
            "No completions should be returned for plain text"
        );
    }
}
