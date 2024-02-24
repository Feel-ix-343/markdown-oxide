use std::{path::Path, iter};

use itertools::Itertools;
use tower_lsp::{lsp_types::{SemanticTokensParams, SemanticTokensResult, SemanticToken, SemanticTokenType}, jsonrpc::Result};

use crate::vault::Vault;


pub fn semantic_tokens_full(
    vault: &Vault,
    path: &Path,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {


    let references_in_file = vault.select_references(Some(path))?;

    let tokens = references_in_file.into_iter()
        .fold(vec![], |acc, (_, reference)| {

            let range = reference.data().range;

            match acc[..] {
                [] => vec![(reference, SemanticToken {
                    delta_line: range.start.line,
                    delta_start: range.start.character,
                    length: range.end.character - range.start.character,
                    token_type: 0,
                    token_modifiers_bitset: 0
                })],
                [.., (prev_ref, _)] => acc.into_iter().chain(iter::once((reference, SemanticToken {
                    delta_line: range.start.line - prev_ref.data().range.start.line,
                    delta_start: range.start.character,
                    length: range.end.character - range.start.character,
                    token_type: 0,
                    token_modifiers_bitset: 0
                }))).collect_vec()
            }
        })
        .into_iter()
        .map(|(_, token)| token)
        .collect_vec(); // TODO: holy this is bad


    return Some(SemanticTokensResult::Tokens(tower_lsp::lsp_types::SemanticTokens { result_id: None, data: tokens }))

}
