use std::fs::File;
use std::path::Path;

use crate::config::Settings;
use chrono::offset::Local;
use chrono::{Days, NaiveDate, NaiveDateTime};
use fuzzydate::parse;
use serde_json::Value;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{MessageType, ShowDocumentParams, Url};

fn datetime_to_file(
    datetime: NaiveDateTime,
    dailynote_format: &str,
    root_dir: &Path,
) -> Option<Url> {
    let filename = datetime.format(dailynote_format).to_string();
    let path = root_dir.join(&filename);

    Url::from_file_path(path.with_extension("md")).ok()
}

fn date_to_file(date: NaiveDate, dailynote_format: &str, root_dir: &Path) -> Option<Url> {
    let filename = date.format(dailynote_format).to_string();
    let path = root_dir.join(&filename);

    Url::from_file_path(path.with_extension("md")).ok()
}

fn extract_date_from_filename(filename: &str, dailynote_format: &str) -> Option<NaiveDate> {
    let filename = filename.strip_suffix(".md").unwrap_or(filename);
    NaiveDate::parse_from_str(filename, dailynote_format).ok()
}

fn parse_relative_directive(input: &str) -> Option<i64> {
    let trimmed = input.trim();

    match trimmed {
        "prev" => Some(-1),
        "next" => Some(1),
        _ => {
            if let Some(stripped) = trimmed.strip_prefix('+') {
                stripped.parse::<i64>().ok()
            } else if let Some(stripped) = trimmed.strip_prefix('-') {
                stripped.parse::<i64>().ok().map(|n| -n)
            } else {
                trimmed.parse::<i64>().ok()
            }
        }
    }
}

pub async fn jump(
    client: &tower_lsp::Client,
    root_dir: &Path,
    settings: &Settings,
    jump_to: Option<&str>,
) -> Result<Option<Value>> {
    let daily_note_format = &settings.dailynote;
    let daily_note_path = root_dir.join(&settings.daily_notes_folder);

    let note_file = match jump_to {
        Some(jmp_str) => {
            if let Some(offset) = parse_relative_directive(jmp_str) {
                let base_date = Local::now().date_naive();

                let target_date = if offset >= 0 {
                    base_date.checked_add_days(Days::new(offset as u64))
                } else {
                    base_date.checked_sub_days(Days::new((-offset) as u64))
                };

                target_date.and_then(|date| date_to_file(date, daily_note_format, &daily_note_path))
            } else {
                parse(jmp_str)
                    .ok()
                    .and_then(|dt| datetime_to_file(dt, daily_note_format, &daily_note_path))
            }
        }
        None => datetime_to_file(
            Local::now().naive_local(),
            daily_note_format,
            &daily_note_path,
        ),
    };

    if let Some(uri) = note_file {
        let _ = uri.to_file_path().map(|path| {
            path.parent().map(std::fs::create_dir_all);

            let _ = File::create_new(path.as_path());
        });

        client
            .show_document(ShowDocumentParams {
                uri,
                external: Some(false),
                take_focus: Some(true),
                selection: None,
            })
            .await
            .map(|success| Some(success.into()))
    } else {
        client
            .log_message(
                MessageType::ERROR,
                format!("could not parse {jump_to:?}: {:?}", jump_to.map(parse)),
            )
            .await;
        Err(Error::invalid_params(format!(
            "Could not parse journal format ({jump_to:?}) as a valid uri: {:?}.",
            jump_to.map(parse)
        )))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use fuzzydate::parse;

    use super::{datetime_to_file, extract_date_from_filename, parse_relative_directive};

    #[test]
    fn test_string_to_file() {
        let input = "today";

        let parsed_datetime = parse(input).unwrap();

        let _ = datetime_to_file(
            parsed_datetime,
            "%Y-%m-%d",
            &std::fs::canonicalize("./").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_parse_relative_directive_prev() {
        assert_eq!(parse_relative_directive("prev"), Some(-1));
    }

    #[test]
    fn test_parse_relative_directive_next() {
        assert_eq!(parse_relative_directive("next"), Some(1));
    }

    #[test]
    fn test_parse_relative_directive_plus() {
        assert_eq!(parse_relative_directive("+1"), Some(1));
        assert_eq!(parse_relative_directive("+7"), Some(7));
        assert_eq!(parse_relative_directive("+30"), Some(30));
    }

    #[test]
    fn test_parse_relative_directive_minus() {
        assert_eq!(parse_relative_directive("-1"), Some(-1));
        assert_eq!(parse_relative_directive("-7"), Some(-7));
        assert_eq!(parse_relative_directive("-30"), Some(-30));
    }

    #[test]
    fn test_parse_relative_directive_plain_number() {
        assert_eq!(parse_relative_directive("1"), Some(1));
        assert_eq!(parse_relative_directive("7"), Some(7));
    }

    #[test]
    fn test_parse_relative_directive_invalid() {
        assert_eq!(parse_relative_directive("invalid"), None);
        assert_eq!(parse_relative_directive("next monday"), None);
        assert_eq!(parse_relative_directive(""), None);
    }

    #[test]
    fn test_extract_date_from_filename() {
        let date = extract_date_from_filename("2024-11-09.md", "%Y-%m-%d");
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2024, 11, 9).unwrap()));

        let date = extract_date_from_filename("2024-11-09", "%Y-%m-%d");
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2024, 11, 9).unwrap()));
    }

    #[test]
    fn test_extract_date_from_filename_invalid() {
        let date = extract_date_from_filename("not-a-date.md", "%Y-%m-%d");
        assert_eq!(date, None);

        let date = extract_date_from_filename("2024-13-01.md", "%Y-%m-%d");
        assert_eq!(date, None);
    }
}
