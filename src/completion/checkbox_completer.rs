use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit,
    InsertTextFormat, Position, Range, TextEdit,
};

use super::{Completable, Completer};

pub struct CheckboxCompleter {
    line: u32,
    character: u32,
    preceding_text: String,
}

impl<'a> Completer<'a> for CheckboxCompleter {
    fn construct(context: super::Context<'a>, line: usize, character: usize) -> Option<Self>
    where
        Self: Sized + Completer<'a>,
    {
        if !context.settings.checkbox_completions {
            return None;
        }

        let line_chars = context.vault.select_line(context.path, line as isize)?;
        let text_up_to_cursor: String = line_chars.into_iter().take(character).collect();

        let trimmed = text_up_to_cursor.trim_start();

        let space_idx = trimmed.find(' ')?;
        let (bullet, rest) = trimmed.split_at(space_idx);
        let rest_trimmed = rest.trim_start();

        let is_valid_bullet = bullet == "-"
            || bullet == "*"
            || bullet == "+"
            || (bullet.ends_with('.')
                && bullet.len() > 1
                && bullet[..bullet.len() - 1]
                    .chars()
                    .all(|c| c.is_ascii_digit()));

        if !is_valid_bullet || !rest_trimmed.starts_with('[') {
            return None;
        }

        // To avoid conflicting with normal links (e.g. `- [Link](...)`),
        // we strictly only trigger if the user typed `[`, `[ `, or `[` followed by a single character.
        // It must not be longer than 2 characters (the bracket and one inner char)
        if rest_trimmed.len() > 2 {
            return None;
        }

        let prefix_len = text_up_to_cursor.len() - rest_trimmed.len();
        let preceding_text = text_up_to_cursor[..prefix_len].to_string();

        Some(Self {
            preceding_text,
            line: line as u32,
            character: character as u32,
        })
    }

    fn completions(&self) -> Vec<impl super::Completable<'a, Self>>
    where
        Self: Sized,
    {
        vec![
            CheckboxCompletion::Unchecked,
            CheckboxCompletion::Checked,
            CheckboxCompletion::Important,
            CheckboxCompletion::InProgress,
            CheckboxCompletion::Scheduled,
            CheckboxCompletion::Rescheduled,
            CheckboxCompletion::Cancelled,
            CheckboxCompletion::Question,
            CheckboxCompletion::Star,
            CheckboxCompletion::Info,
            CheckboxCompletion::Quote,
        ]
    }

    type FilterParams = &'static str;
    fn completion_filter_text(&self, params: Self::FilterParams) -> String {
        format!("{}[{}]", self.preceding_text, params)
    }
}

enum CheckboxCompletion {
    Unchecked,
    Checked,
    InProgress,
    Rescheduled,
    Cancelled,
    Scheduled,
    Important,
    Question,
    Star,
    Info,
    Quote,
}

impl Completable<'_, CheckboxCompleter> for CheckboxCompletion {
    fn completions(&self, completer: &CheckboxCompleter) -> Option<CompletionItem> {
        let (name, char_val, sort_idx) = match self {
            Self::Unchecked => ("Unchecked", " ", "00"),
            Self::Checked => ("Checked", "x", "01"),
            Self::Important => ("Important", "!", "02"),
            Self::InProgress => ("In Progress", "/", "03"),
            Self::Scheduled => ("Scheduled", "<", "04"),
            Self::Rescheduled => ("Rescheduled", ">", "05"),
            Self::Cancelled => ("Cancelled", "-", "06"),
            Self::Question => ("Question", "?", "07"),
            Self::Star => ("Star", "*", "08"),
            Self::Info => ("Info", "i", "09"),
            Self::Quote => ("Quote", "\"", "10"),
        };

        let snippet = format!("{}[{}] ${{1}}", completer.preceding_text, char_val);
        let filter_text = completer.completion_filter_text(char_val);

        let completion_item = CompletionItem {
            label: format!("[{}]", char_val),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(format!(" {}", name)),
                description: None,
            }),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            kind: Some(CompletionItemKind::SNIPPET),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: Range {
                    start: Position {
                        line: completer.line,
                        character: 0,
                    },
                    end: Position {
                        line: completer.line,
                        character: completer.character,
                    },
                },
                new_text: snippet,
            })),
            filter_text: Some(filter_text),
            sort_text: Some(sort_idx.to_string()),
            ..Default::default()
        };

        Some(completion_item)
    }
}
