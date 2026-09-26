use std::{collections::HashSet, iter, path::Path};

use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use ropey::Rope;
use tower_lsp::lsp_types::{
    Position, Range, SemanticToken, SemanticTokensParams, SemanticTokensResult,
};

use crate::{config::Settings, diagnostics::path_unresolved_references, vault::Vault};

/// LSP semantic tokens are single-line. A wrapped `[text](url)` has
/// `end.character < start.character`, so subtracting the columns underflows
/// `u32` in release (the client then spins on a ~4e9-length token — #466).
fn semantic_token_length(range: Range, rope: Option<&Rope>) -> u32 {
    if range.end.line == range.start.line {
        return range
            .end
            .character
            .saturating_sub(range.start.character);
    }

    let Some(rope) = rope else {
        return 1;
    };
    let line_idx = range.start.line as usize;
    if line_idx >= rope.len_lines() {
        return 1;
    }
    let line = rope.line(line_idx);
    let mut line_chars = line.len_chars() as u32;
    if line.get_char(line.len_chars().saturating_sub(1)) == Some('\n') {
        line_chars = line_chars.saturating_sub(1);
    }
    if line.get_char(line.len_chars().saturating_sub(1)) == Some('\r') {
        line_chars = line_chars.saturating_sub(1);
    }
    line_chars.saturating_sub(range.start.character).max(1)
}

fn saturating_delta_start(curr: Position, prev: Position) -> u32 {
    if curr.line == prev.line {
        curr.character.saturating_sub(prev.character)
    } else {
        curr.character
    }
}

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
    let rope = vault.ropes.get(path);

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
            let length = semantic_token_length(range.0, rope);

            let is_unresolved = path_unresolved
                .as_ref()
                .is_some_and(|unresolved| unresolved.contains(reference));

            match acc[..] {
                [] => vec![(
                    reference,
                    SemanticToken {
                        delta_line: range.start.line,
                        delta_start: range.start.character,
                        length,
                        token_type: if is_unresolved { 1 } else { 0 },
                        token_modifiers_bitset: 0,
                    },
                )],
                [.., (prev_ref, _)] => acc
                    .into_iter()
                    .chain(iter::once((
                        reference,
                        SemanticToken {
                            delta_line: range
                                .start
                                .line
                                .saturating_sub(prev_ref.data().range.start.line),
                            delta_start: saturating_delta_start(
                                range.start,
                                prev_ref.data().range.start,
                            ),
                            length,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    use tower_lsp::lsp_types::{
        ClientCapabilities, PartialResultParams, Position, Range, SemanticTokens,
        SemanticTokensParams, TextDocumentIdentifier, Url, WorkDoneProgressParams,
    };

    fn settings() -> Settings {
        Settings::new(Path::new("."), &ClientCapabilities::default()).unwrap()
    }

    #[test]
    fn wrapped_link_token_length_does_not_underflow() {
        // Naive `end.character - start.character` wraps u32 when the link
        // text breaks across a line (end column is smaller than start).
        let text = "# Lorem\n\nSed ut perspiciatis unde omnis iste natus error (see [natus\nerror](#sit)) sit.\n";
        let rope = Rope::from_str(text);
        let range = Range {
            start: Position {
                line: 2,
                character: 40,
            },
            end: Position {
                line: 3,
                character: 5,
            },
        };
        let len = semantic_token_length(range, Some(&rope));
        assert!(
            len > 0 && len < 200,
            "expected first-line remainder, got {len}"
        );
    }

    #[test]
    fn parenthetical_wrapped_md_link_semantic_tokens_are_bounded() {
        // https://github.com/Feel-ix-343/markdown-oxide/issues/466
        let text = "# Lorem Ipsum\n\nSed ut perspiciatis unde omnis iste natus error (see [natus\nerror](#sit)) sit voluptatem accusantium doloremque laudantium.\n";
        let dir = std::env::temp_dir().join("mdoxide-issue-466");
        fs::create_dir_all(&dir).unwrap();
        let path: PathBuf = dir.join("hang.md");
        fs::write(&path, text).unwrap();

        let vault = Vault::construct_vault(&settings(), &dir).unwrap();
        let params = SemanticTokensParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse("file:///hang.md").unwrap(),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = semantic_tokens_full(&vault, &path, params, &settings())
            .expect("semantic tokens enabled by default");
        let SemanticTokensResult::Tokens(SemanticTokens { data, .. }) = result else {
            panic!("expected a token list");
        };
        assert!(
            data.iter().all(|token| token.length < 10_000),
            "wrapped parenthetical link produced a huge token length: {data:?}"
        );
    }
}
