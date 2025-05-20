use crate::config::Settings;

pub fn filename_is_formatted(context: &Settings, filename: &str) -> bool {
    let try_parsed = chrono::NaiveDate::parse_from_str(filename, &context.dailynote);

    try_parsed.is_ok()
}
