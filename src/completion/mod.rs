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

use self::{link_completer::MarkdownLinkCompleter, matcher::fuzzy_match, tag_completer::TagCompleter, unindexed_block_completer::UnindexedBlockCompleter};
use self::link_completer::WikiLinkCompleter;

mod link_completer;
mod matcher;
mod unindexed_block_completer;
mod tag_completer;


#[derive(Clone, Copy)]
pub struct Context<'a>{
    vault: &'a Vault,
    opened_files: &'a [PathBuf],
}

pub trait Completer<'a> : Sized {
    fn construct(context: Context<'a>, path: &Path, line: usize, character: usize) -> Option<Self>
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

#[derive(Debug, PartialEq, Eq)]
pub struct CompletableTag {
    full_range: LineRange<usize>,
    /// Tag name and range not including the '#'
    inputted_tag: (String, LineRange<usize>)
}

fn get_completable_tag(line: &Vec<char>, cursor_character: usize) -> Option<CompletableTag> {
    static PARTIAL_TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\#(?<text>[a-zA-Z0-9\/]*)").unwrap()
    }); 

    let line_string = String::from_iter(line);

    let captures_iter = PARTIAL_TAG_REGEX.captures_iter(&line_string);

    return captures_iter
        .flat_map(|captures| {

            let (full, tag_text) = (
                captures.get(0)?,
                captures.name("text")?,
            );

            // check if the cursor is in the tag
            let preceding_character = cursor_character - 1; // User is inserting into the position after the character they are looking at; "#tag|"  cursor is a position 4; I want pos 3; the end of the tag
            if preceding_character >= full.range().start && preceding_character < full.range().end { // end is exclusive
                return Some(CompletableTag {
                    full_range: full.range(),
                    inputted_tag: (tag_text.as_str().to_string(), tag_text.range())
                })
            } else {
                return None
            }

        })
        .next()



}

pub fn get_completions(
    vault: &Vault,
    initial_completion_files: &[PathBuf],
    params: &CompletionParams,
    _path: &Path,
) -> Option<CompletionResponse> {
    let Ok(path) = params
        .text_document_position
        .text_document
        .uri
        .to_file_path()
    else {
        return None;
    };

    let completion_context = Context {
        vault,
        opened_files: initial_completion_files
    };

    run_completer::<UnindexedBlockCompleter<MarkdownLinkCompleter>>(completion_context, &path, params.text_document_position.position.line, params.text_document_position.position.character)
        .or_else(|| run_completer::<UnindexedBlockCompleter<WikiLinkCompleter>>(completion_context, &path, params.text_document_position.position.line, params.text_document_position.position.character))
        .or_else(|| run_completer::<MarkdownLinkCompleter>(completion_context, &path, params.text_document_position.position.line, params.text_document_position.position.character))
        .or_else(|| run_completer::<WikiLinkCompleter>(completion_context, &path, params.text_document_position.position.line, params.text_document_position.position.character))
        .or_else(|| run_completer::<TagCompleter>(completion_context, &path, params.text_document_position.position.line, params.text_document_position.position.character))

}
//
//     if let Some(index) = get_wikilink_index(&selected_line, character) {
//
//         // completions for wikilinks `[[text|` where | is the cursor
//         let range = Range {
//             start: Position {
//                 line: line as u32,
//                 character: index as u32 + 1,
//             },
//             end: Position {
//                 line: line as u32,
//                 character: character as u32,
//             },
//         };
//
//         let cmp_text = selected_line.get(index + 1..character)?;
//
//         return match *cmp_text {
//             [] => Some(CompletionResponse::List(CompletionList {
//                 items: initial_completion_files
//                     .iter()
//                     .map(|path| {
//                         match std::fs::metadata(path).and_then(|meta| meta.modified()) {
//                             Ok(modified) => (path, modified),
//                             Err(_) => (path, SystemTime::UNIX_EPOCH),
//                         }
//                     })
//                     .sorted_by_key(|(_, modified)| *modified)
//                     .take(15)
//                     .filter_map(|(path_i, _)| {
//                         Some(
//                             vault
//                                 .select_referenceable_nodes(Some(path_i))
//                                 .into_iter()
//                                 .filter(|referenceable| {
//                                     !matches!(referenceable, Referenceable::Tag(_, _))
//                                     && !matches!(
//                                         referenceable,
//                                         Referenceable::Footnote(_, _)
//                                     )
//                                 })
//                                 .collect_vec(),
//                         )
//                     })
//                     .flatten()
//                     .filter_map(|referenceable| {
//                         default_completion_item(vault, &referenceable, None)
//                     })
//                     .collect::<Vec<_>>(),
//                 is_incomplete: true,
//             })),
//             [' ', ref text @ ..] if !text.contains(&']') => {
//                 let blocks = vault.select_blocks();
//
//                 let matches = fuzzy_match(&String::from_iter(text), blocks);
//
//                 let rand_id = nanoid!(
//                     5,
//                     &[
//                         'a', 'b', 'c', 'd', 'e', 'f', 'g', '1', '2', '3', '4', '5', '6', '7', '8',
//                         '9'
//                     ]
//                 );
//
//                 return Some(CompletionResponse::List(CompletionList {
//                     is_incomplete: true,
//                     items: matches
//                         .into_par_iter()
//                         .take(50)
//                         .filter(|(block, _)| {
//                             String::from_iter(selected_line.clone()).trim() != block.text
//                         })
//                         .flat_map(|(block, rank)| {
//                             let path_ref = get_obsidian_ref_path(vault.root_dir(), &block.file)?;
//                             let file_name = block.file.file_stem()?.to_str()?;
//
//                             let url = Url::from_file_path(&block.file).ok()?;
//                             Some(CompletionItem {
//                                 label: block.text.clone(),
//                                 sort_text: Some(rank.to_string()),
//                                 documentation: Some(Documentation::MarkupContent(MarkupContent {
//                                     kind: MarkupKind::Markdown,
//                                     value: (block.range.start.line as isize - 5
//                                         ..=block.range.start.line as isize + 5)
//                                         .flat_map(|i| Some((vault.select_line(&block.file, i)?, i)))
//                                         .map(|(iter, ln)| {
//                                             if ln == block.range.start.line as isize {
//                                                 format!("**{}**\n", String::from_iter(iter).trim())
//                                             // highlight the block to be references
//                                             } else {
//                                                 String::from_iter(iter)
//                                             }
//                                         })
//                                         .join(""),
//                                 })),
//                                 filter_text: Some(format!(" {}", block.text)),
//                                 text_edit: Some(CompletionTextEdit::Edit(TextEdit {
//                                     range,
//                                     new_text: format!("{}#^{}", file_name, rand_id),
//                                 })),
//                                 command: Some(Command {
//                                     title: "Insert Block Reference Into File".into(),
//                                     command: "apply_edits".into(),
//                                     arguments: Some(vec![serde_json::to_value(
//                                         tower_lsp::lsp_types::WorkspaceEdit {
//                                             changes: Some(
//                                                 vec![(
//                                                     url,
//                                                     vec![TextEdit {
//                                                         range: Range {
//                                                             start: Position {
//                                                                 line: block.range.end.line,
//                                                                 character: block
//                                                                     .range
//                                                                     .end
//                                                                     .character
//                                                                     - 1,
//                                                             },
//                                                             end: Position {
//                                                                 line: block.range.end.line,
//                                                                 character: block
//                                                                     .range
//                                                                     .end
//                                                                     .character
//                                                                     - 1,
//                                                             },
//                                                         },
//                                                         new_text: format!("   ^{}", rand_id),
//                                                     }],
//                                                 )]
//                                                 .into_iter()
//                                                 .collect(),
//                                             ),
//                                             change_annotations: None,
//                                             document_changes: None,
//                                         },
//                                     )
//                                     .ok()?]),
//                                 }),
//                                 ..Default::default()
//                             })
//                         })
//                         .collect(),
//                 }));
//             }
//             ref filter_text @ [..] if !filter_text.contains(&']') => {
//                 let all_links = MatchableReferenceable::from_vault(vault);
//                 let matches = fuzzy_match(&String::from_iter(filter_text), all_links);
//
//                 return Some(CompletionResponse::List(CompletionList {
//                     is_incomplete: true,
//                     items: matches
//                         .into_iter()
//                         .take(30)
//                         .filter(|(MatchableReferenceable(r, name), _)| {
//                             !(*name == String::from_iter(filter_text) && matches!(r, Referenceable::UnresovledFile(..) | Referenceable::UnresolvedHeading(..) | Referenceable::UnresovledIndexedBlock(..)))
//                         })
//                         .filter_map(|(MatchableReferenceable(referenceable, _), rank)| {
//                             default_completion_item(
//                                 vault,
//                                 &referenceable,
//                                 Some(CompletionTextEdit::Edit(TextEdit {
//                                     range,
//                                     new_text: referenceable.get_refname(&vault.root_dir())?.file_refname()?
//                                 })),
//                             )
//                             .and_then(|item| Some(CompletionItem {
//                                     sort_text: Some(rank.to_string()),
//                                     filter_text: Some(referenceable.get_refname(&vault.root_dir())?.to_string()),
//                                     ..item
//                             }))
//                         })
//                         .collect::<Vec<_>>(),
//                 }));
//             }
//             _ => None,
//         };
//     } else if let Some(partialmdlink) = get_completable_mdlink(&selected_line, character) {
//         match partialmdlink {
//             CompletableMDLink {
//                 path,
//                 infile_ref,
//                 full_range,
//                 display,
//                 partial,
//             } => {
//                 let inputted_refname = format!(
//                     "{}{}",
//                     path.0,
//                     infile_ref
//                         .clone()
//                         .map(|(string, _)| format!("#{}", string))
//                         .unwrap_or("".to_string())
//                 );
//
//
//                 let all_links = MatchableReferenceable::from_vault(vault);
//
//                 let matches = fuzzy_match(&inputted_refname, all_links);
//
//                 return Some(CompletionResponse::List(CompletionList {
//                     is_incomplete: true,
//                     items: matches
//                         .into_iter()
//                         .take(50)
//                         .filter(|(MatchableReferenceable(r, name), _)| 
//                             !(*name == inputted_refname && matches!(r, Referenceable::UnresovledFile(..) | Referenceable::UnresolvedHeading(..) | Referenceable::UnresovledIndexedBlock(..)))
//                         )
//                         .flat_map(|(MatchableReferenceable(referenceable, _), rank)| {
//                             default_completion_item(
//                                 vault,
//                                 &referenceable,
//                                 Some(CompletionTextEdit::Edit(TextEdit {
//                                     range: Range {
//                                         start: Position {
//                                             line: line as u32,
//                                             character: full_range.start as u32,
//                                         },
//                                         end: Position {
//                                             line: line as u32,
//                                             character: full_range.end as u32,
//                                         },
//                                     },
//                                     new_text: format!(
//                                         "[${{1:{}}}]({}{}{}{})",
//                                         match (
//                                             display.0.as_str(),
//                                             referenceable.get_refname(vault.root_dir())?.infile_ref
//                                         ) {
//                                             ("", Some(infile_ref_text)) => infile_ref_text.clone(),
//                                             ("", None) => {
//                                                 match referenceable {
//                                                     Referenceable::File(_, mdfile) => {
//                                                         match mdfile.headings.first() {
//                                                             Some(heading) => {
//                                                                 heading.heading_text.clone()
//                                                             }
//                                                             None => "".to_string(),
//                                                         }
//                                                     }
//
//                                                     _ => "".to_string(),
//                                                 }
//                                             }
//                                             (display_text, _) => display_text.to_string(),
//                                         },
//                                         if referenceable
//                                             .get_refname(vault.root_dir())?
//                                             .path?
//                                             .contains(" ")
//                                         {
//                                             "<"
//                                         } else {
//                                             ""
//                                         },
//                                         referenceable
//                                             .get_refname(vault.root_dir())?
//                                             .link_file_key()?,
//                                         match referenceable
//                                             .get_refname(vault.root_dir())?
//                                             .infile_ref
//                                         {
//                                             Some(string) => format!("#{}", string),
//                                             None => "".to_string(),
//                                         },
//                                         if referenceable
//                                             .get_refname(vault.root_dir())?
//                                             .path?
//                                             .contains(" ")
//                                         {
//                                             ">"
//                                         } else {
//                                             ""
//                                         },
//                                     ),
//                                 })),
//                             )
//                             .and_then(|item| {
//                                 Some(CompletionItem {
//                                     label: format!("{}{}", 
//                                         referenceable.get_refname(vault.root_dir())?.link_file_key()?,
//                                         referenceable.get_refname(vault.root_dir())?.infile_ref.map(|thing| format!("#{}", thing)).unwrap_or("".into())
//                                     ),
//                                     sort_text: Some(rank.to_string()),
//                                     insert_text_format: Some(InsertTextFormat::SNIPPET),
//                                     filter_text: Some(format!(
//                                         "[{}]({}",
//                                         display.0,
//                                         referenceable.get_refname(vault.root_dir())?.to_string()
//                                     )),
//                                     ..item
//                                 })
//                             })
//                         })
//                         .collect::<Vec<_>>(),
//                 }));
//             }
//         }
//     } else if let Some(CompletableTag{ full_range, inputted_tag: (completable_tag_name, tag_name_range) }) = get_completable_tag(&selected_line, character) {
//         // Initial Tag completion
//         let tag_refereneables =
//             vault
//                 .select_referenceable_nodes(None)
//                 .into_iter()
//                 .flat_map(|referenceable| match referenceable {
//                     tag @ Referenceable::Tag(_, _) => Some(tag),
//                     _ => None,
//                 })
//                 .flat_map(|tag| Some(MatchableReferenceable(tag.clone(), tag.get_refname(&vault.root_dir())?.path?)))
//                 .collect_vec();
//
//         let matches = fuzzy_match(&completable_tag_name, tag_refereneables);
//
//         return Some(CompletionResponse::List(CompletionList {
//             is_incomplete: true,
//             items: matches
//                 .into_iter()
//                 .take(20)
//                 .filter(|(MatchableReferenceable(_, tag_name), _)| *tag_name != completable_tag_name)
//                 .flat_map(|(MatchableReferenceable(tag, tag_name), ranking)| {
//                     default_completion_item(vault, &tag, Some(CompletionTextEdit::Edit(TextEdit {
//                         new_text: format!("#{}", tag_name.clone()),
//                         range: Range {
//                             start: Position {
//                                 line: line as u32,
//                                 character: full_range.start as u32,
//                             },
//                             end: Position {
//                                 line: line as u32,
//                                 character: full_range.end as u32,
//                             },
//                         }
//                     })))
//                         .map(|item| CompletionItem {
//                             label: tag_name.clone(),
//                             sort_text: Some(ranking.to_string()),
//                             filter_text: Some(format!("#{}", tag_name)),
//                             ..item
//                         })
//                 })
//                 .unique_by(|c| c.label.to_owned())
//                 .collect_vec(),
//         }));
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


fn run_completer<'a, T: Completer<'a>>(context: Context<'a>, path: &Path, line: u32, character: u32) -> Option<CompletionResponse> {

    let completer = T::construct(context, path, line as usize, character as usize)?;
    let completions = completer.completions();

    let completions = completions
        .into_iter()
        .take(50)
        .map(|completable| completable.completions(&completer).collect::<Vec<_>>().into_iter()) // Hate this
        .flatten()
        .collect::<Vec<CompletionItem>>();

    Some(CompletionResponse::List(CompletionList { is_incomplete: true, items: completions }))

}

