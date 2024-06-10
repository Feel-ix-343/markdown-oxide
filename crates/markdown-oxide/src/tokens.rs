use std::{collections::HashSet, iter, path::Path};

use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tower_lsp::lsp_types::{SemanticToken, SemanticTokensParams, SemanticTokensResult};

use moxide_config::Settings;
use vault::Vault;

use crate::diagnostics::path_unresolved_references;

pub fn semantic_tokens_full(
    vault: &Vault,
    path: &Path,
    _params: SemanticTokensParams,
    settings: &Settings,
) -> Option<SemanticTokensResult> {
    if !settings.semantic_tokens {
        return None;
    }

    let references_in_file = vault.select_references(Some(path))?;

    let path_unresolved: Option<HashSet<_>> =
        path_unresolved_references(vault, path).map(|thing| {
            thing
                .into_par_iter()
                .map(|(_, reference)| reference)
                .collect()
        });

    let tokens = references_in_file
        .into_iter()
        .sorted_by_key(|(_, reference)| {
            (
                reference.data().range.start.line,
                reference.data().range.start.character,
            )
        })
        .fold(vec![], |acc, (_path, reference)| {
            let range = reference.data().range;

            let is_unresolved = path_unresolved
                .as_ref()
                .is_some_and(|unresolved| unresolved.contains(reference));

            match acc[..] {
                [] => vec![(
                    reference,
                    SemanticToken {
                        delta_line: range.start.line,
                        delta_start: range.start.character,
                        length: range.end.character - range.start.character,
                        token_type: if is_unresolved { 1 } else { 0 },
                        token_modifiers_bitset: 0,
                    },
                )],
                [.., (prev_ref, _)] => acc
                    .into_iter()
                    .chain(iter::once((
                        reference,
                        SemanticToken {
                            delta_line: range.start.line - prev_ref.data().range.start.line,
                            delta_start: if range.start.line == prev_ref.data().range.start.line {
                                range.start.character - prev_ref.data().range.start.character
                            } else {
                                range.start.character
                            },
                            length: range.end.character - range.start.character,
                            token_type: if is_unresolved { 1 } else { 0 },
                            token_modifiers_bitset: 0,
                        },
                    )))
                    .collect_vec(),
            }
        })
        .into_par_iter()
        .map(|(_, token)| token)
        .collect::<Vec<_>>(); // TODO: holy this is bad

    Some(SemanticTokensResult::Tokens(
        tower_lsp::lsp_types::SemanticTokens {
            result_id: None,
            data: tokens,
        },
    ))
}
