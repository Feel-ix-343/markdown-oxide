use std::{iter::once, path::{Path, PathBuf}, time::SystemTime};

use itertools::Itertools;
use nanoid::nanoid;

use nucleo_matcher::{
    pattern::{self, Normalization},
    Matcher,
};
use once_cell::sync::Lazy;
use rayon::prelude::*;

use regex::Regex;
use tower_lsp::lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionList, CompletionParams, CompletionResponse, CompletionTextEdit, Documentation, InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind, Position, Range, TextEdit, Url
};

use crate::{
    ui::preview_referenceable,
    vault::{
        get_obsidian_ref_path, Block, MyRange, Preview, Reference, Referenceable, Refname, Vault, MDTag,
    },
};

use self::{footnote_completer::FootnoteCompleter, link_completer::MarkdownLinkCompleter, matcher::fuzzy_match, tag_completer::TagCompleter, unindexed_block_completer::UnindexedBlockCompleter};
use self::link_completer::WikiLinkCompleter;

mod link_completer;
mod matcher;
mod unindexed_block_completer;
mod tag_completer;
mod footnote_completer;


#[derive(Clone, Copy)]
pub struct Context<'a>{
    vault: &'a Vault,
    opened_files: &'a [PathBuf],
    path: &'a Path
}

pub trait Completer<'a> : Sized {
    fn construct(context: Context<'a>, line: usize, character: usize) -> Option<Self>
    where Self: Sized + Completer<'a>;

    fn completions(&self) -> Vec<impl Completable<'a, Self>> where Self: Sized;

    type FilterParams;
    /// Completere like nvim-cmp are odd so manually define the filter text as a situational workaround
    fn completion_filter_text(&self, params: Self::FilterParams) -> String;

    // fn compeltion_resolve(&self, vault: &Vault, resolve_item: CompletionItem) -> Option<CompletionItem>;
}


pub trait Completable<'a, T: Completer<'a>> : Sized {
    fn completions(&self, completer: &T) -> impl Iterator<Item = CompletionItem>;
}

/// Range indexes for one line of the file; NOT THE WHOLE FILE
type LineRange<T> = std::ops::Range<T>;


pub fn get_completions(
    vault: &Vault,
    initial_completion_files: &[PathBuf],
    params: &CompletionParams,
    path: &Path,
) -> Option<CompletionResponse> {
    let completion_context = Context {
        vault,
        opened_files: initial_completion_files,
        path: &path,
    };

    run_completer::<UnindexedBlockCompleter<MarkdownLinkCompleter>>(completion_context, params.text_document_position.position.line, params.text_document_position.position.character)
        .or_else(|| run_completer::<UnindexedBlockCompleter<WikiLinkCompleter>>(completion_context, params.text_document_position.position.line, params.text_document_position.position.character))
        .or_else(|| run_completer::<MarkdownLinkCompleter>(completion_context, params.text_document_position.position.line, params.text_document_position.position.character))
        .or_else(|| run_completer::<WikiLinkCompleter>(completion_context, params.text_document_position.position.line, params.text_document_position.position.character))
        .or_else(|| run_completer::<TagCompleter>(completion_context, params.text_document_position.position.line, params.text_document_position.position.character))
        .or_else(|| run_completer::<FootnoteCompleter>(completion_context, params.text_document_position.position.line, params.text_document_position.position.character))

}
//     } else if character
//         .checked_sub(1)
//         .and_then(|start| selected_line.get(start..character))
//         == Some(&['['])
//     {
//         let footnote_referenceables = vault
//             .select_referenceable_nodes(Some(&path))
//             .into_iter()
//             .flat_map(|referenceable| match referenceable {
//                 Referenceable::Footnote(footnote_path, _)
//                     if footnote_path.as_path() == path.as_path() =>
//                 {
//                     Some(referenceable)
//                 }
//                 _ => None,
//             });
//
//         return Some(CompletionResponse::Array(
//             footnote_referenceables
//                 .filter_map(|footnote| {
//                     footnote
//                         .get_refname(vault.root_dir())
//                         .map(|root| CompletionItem {
//                             kind: Some(CompletionItemKind::REFERENCE),
//                             label: root.clone(),
//                             documentation: preview_referenceable(vault, &footnote)
//                                 .map(Documentation::MarkupContent),
//                             filter_text: vault
//                                 .select_referenceable_preview(&footnote)
//                                 .and_then(|preview| match preview {
//                                     Preview::Text(string) => Some(string),
//                                     Preview::Empty => None,
//                                 })
//                                 .map(|preview_string| format!("{}{}", *root, &preview_string)),
//                             ..Default::default()
//                         })
//                 })
//                 .unique_by(|c| c.label.to_owned())
//                 .collect_vec(),
//         ));
//     } else {
//         return None;
//     }
// }
//
// fn default_completion_item(
//     vault: &Vault,
//     referenceable: &Referenceable,
//     text_edit: Option<CompletionTextEdit>,
// ) -> Option<CompletionItem> {
//     let refname = referenceable.get_refname(vault.root_dir())?;
//     let completion = CompletionItem {
//         kind: match &referenceable {
//             Referenceable::File(..) => Some(CompletionItemKind::FILE),
//             Referenceable::Heading(..) 
//                 | Referenceable::IndexedBlock(..)
//                 | Referenceable::Footnote(..)
//                 | Referenceable::LinkRefDef(..)
//                 => Some(CompletionItemKind::REFERENCE),
//             Referenceable::UnresovledFile(..)
//                 | Referenceable::UnresolvedHeading(..)
//                 | Referenceable::UnresovledIndexedBlock(..)
//                 => Some(CompletionItemKind::KEYWORD),
//             Referenceable::Tag(..) => Some(CompletionItemKind::CONSTANT),
//         },
//         label: refname.file_refname()?,
//         label_details: match referenceable.is_unresolved() {
//             true => Some(CompletionItemLabelDetails {
//                 detail: Some("Unresolved".into()),
//                 description: None,
//             }),
//             false => None,
//         },
//         text_edit,
//         documentation: preview_referenceable(vault, referenceable)
//             .map(Documentation::MarkupContent),
//         ..Default::default()
//     };
//
//     Some(completion)
// }
//
// struct MatchableReferenceable<'a>(Referenceable<'a>, String);
//
// impl MatchableReferenceable<'_> {
//     fn from_vault<'a>(vault: &'a Vault) -> Vec<MatchableReferenceable<'a>> {
//         let all_links = vault
//             .select_referenceable_nodes(None)
//             .into_par_iter()
//             .filter(|referenceable| {
//                 !matches!(referenceable, Referenceable::Tag(..))
//                     && !matches!(referenceable, Referenceable::Footnote(..))
//             })
//             .filter_map(|referenceable| {
//                 referenceable
//                     .get_refname(vault.root_dir())
//                     .map(|string| MatchableReferenceable(referenceable, string.to_string()))
//             })
//             .collect::<Vec<_>>();
//
//         all_links
//     }
// }
//
//
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


fn run_completer<'a, T: Completer<'a>>(context: Context<'a>, line: u32, character: u32) -> Option<CompletionResponse> {

    let completer = T::construct(context, line as usize, character as usize)?;
    let completions = completer.completions();

    let completions = completions
        .into_iter()
        .take(50)
        .map(|completable| completable.completions(&completer).collect::<Vec<_>>().into_iter()) // Hate this
        .flatten()
        .collect::<Vec<CompletionItem>>();

    Some(CompletionResponse::List(CompletionList { is_incomplete: true, items: completions }))

}

