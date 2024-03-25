use std::{iter, path::Path};

use itertools::Itertools;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokensParams, SemanticTokensResult};

use crate::vault::{Referenceable, Vault};

pub fn semantic_tokens_full(
    vault: &Vault,
    path: &Path,
    _params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let references_in_file = vault.select_references(Some(path))?;

    let tokens = references_in_file
        .into_iter()
        .sorted_by_key(|(_, reference)| {
            (
                reference.data().range.start.line,
                reference.data().range.start.character,
            )
        })
        .fold(vec![], |acc, (path, reference)| {
            let range = reference.data().range;

            let is_unresolved = vault.select_referenceables_for_reference(reference, path)
                .into_iter()
                .any(|referenceable| referenceable.is_unresolved());

            match acc[..] {
                [] => vec![(
                    reference,
                    SemanticToken {
                        delta_line: range.start.line,
                        delta_start: range.start.character,
                        length: range.end.character - range.start.character,
                        token_type: if is_unresolved { 1 } else { 0 },
                        token_modifiers_bitset: 0
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
                            token_modifiers_bitset: 0
                        },
                    )))
                    .collect_vec(),
            }
        })
        .into_iter()
        .map(|(_, token)| token)
        .collect_vec(); // TODO: holy this is bad

    Some(SemanticTokensResult::Tokens(
        tower_lsp::lsp_types::SemanticTokens {
            result_id: None,
            data: tokens,
        },
    ))
}
