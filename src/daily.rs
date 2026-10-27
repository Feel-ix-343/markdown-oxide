use crate::config::Settings;
use chrono::NaiveDate;

/// Returns the filename component of the daily note format (the segment after the last '/').
pub fn daily_note_filename_format(dailynote_format: &str) -> &str {
    dailynote_format.rsplit('/').next().unwrap_or(dailynote_format)
}

/// Parses a date from a filename, link text, or relative path using the daily note format setting.
/// First attempts parsing with the full format, then falls back to parsing with just
/// the filename format component if the format contains folder separators.
pub fn parse_daily_note_date(context: &Settings, filename: &str) -> Option<NaiveDate> {
    parse_daily_note_date_from_format(filename, &context.dailynote)
}

/// Parses a date from a filename, link text, or relative path using an explicit format string.
pub fn parse_daily_note_date_from_format(filename: &str, dailynote_format: &str) -> Option<NaiveDate> {
    let filename = filename.strip_suffix(".md").unwrap_or(filename);

    if let Ok(date) = NaiveDate::parse_from_str(filename, dailynote_format) {
        return Some(date);
    }

    let fn_format = daily_note_filename_format(dailynote_format);
    if fn_format != dailynote_format {
        if let Ok(date) = NaiveDate::parse_from_str(filename, fn_format) {
            return Some(date);
        }
    }

    None
}

#[allow(dead_code)]
pub fn filename_is_formatted(context: &Settings, filename: &str) -> bool {
    parse_daily_note_date(context, filename).is_some()
}

/// Formats the relative path for a daily note under the daily notes folder (preserving subfolder structure).
pub fn daily_note_relative_path(context: &Settings, date: NaiveDate) -> String {
    date.format(&context.dailynote).to_string()
}

/// Formats the reference / note name for a daily note (using the filename component).
pub fn daily_note_ref_name(context: &Settings, date: NaiveDate) -> String {
    let fn_format = daily_note_filename_format(&context.dailynote);
    date.format(fn_format).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_format_without_folders() {
        assert_eq!(daily_note_filename_format("%Y-%m-%d"), "%Y-%m-%d");
    }

    #[test]
    fn test_filename_format_with_folders() {
        assert_eq!(daily_note_filename_format("%Y/%m/%Y-%m-%d"), "%Y-%m-%d");
        assert_eq!(daily_note_filename_format("%Y/%m-%B/%Y-%m-%d"), "%Y-%m-%d");
    }

    #[test]
    fn test_parse_daily_note_date_simple() {
        let date = parse_daily_note_date_from_format("2026-10-26", "%Y-%m-%d");
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()));

        let date_md = parse_daily_note_date_from_format("2026-10-26.md", "%Y-%m-%d");
        assert_eq!(date_md, Some(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()));
    }

    #[test]
    fn test_parse_daily_note_date_with_folder_format() {
        let format = "%Y/%m/%Y-%m-%d";

        // Bare filename matching filename component
        let date_bare = parse_daily_note_date_from_format("2026-10-26", format);
        assert_eq!(date_bare, Some(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()));

        let date_bare_md = parse_daily_note_date_from_format("2026-10-26.md", format);
        assert_eq!(date_bare_md, Some(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()));

        // Full relative path matching full format
        let date_full = parse_daily_note_date_from_format("2026/10/2026-10-26", format);
        assert_eq!(date_full, Some(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()));

        let date_full_md = parse_daily_note_date_from_format("2026/10/2026-10-26.md", format);
        assert_eq!(date_full_md, Some(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()));

        // Invalid text returns None
        assert_eq!(parse_daily_note_date_from_format("some-other-file", format), None);
        assert_eq!(parse_daily_note_date_from_format("2026-99-99", format), None);
    }

    #[test]
    fn test_daily_note_relative_path_and_ref_name() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();

        let simple_format = "%Y-%m-%d";
        assert_eq!(date.format(simple_format).to_string(), "2026-10-26");
        assert_eq!(date.format(daily_note_filename_format(simple_format)).to_string(), "2026-10-26");

        let folder_format = "%Y/%m/%Y-%m-%d";
        assert_eq!(date.format(folder_format).to_string(), "2026/10/2026-10-26");
        assert_eq!(date.format(daily_note_filename_format(folder_format)).to_string(), "2026-10-26");
    }
}
