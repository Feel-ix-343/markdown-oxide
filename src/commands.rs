use chrono::NaiveDateTime;
use chrono::offset::Local;
use crate::config::Settings;
use fuzzydate::parse;
use tower_lsp::lsp_types::{ShowDocumentParams, Url};


// fn file_to_datetime(filename: &str, format: &str) -> Result<NaiveDateTime> {
//     // default to the beginning of an underspecified interval,
//     // top of hour, beginning of day, start of week
//
//     let items = StrftimeItems::new(format);
//     let mut parsed = Parsed::new();
//     parse(&mut parsed, "2024-04-01", items);
//     parsed.to_naive_datetime_with_offset(0)
// }


fn datetime_to_file(datetime: NaiveDateTime, format: &str) -> Option<Url> {
    Url::parse(&format!("file:///home/kxnr/wiki/journal/{}", &datetime.format(format).to_string())).ok()
}

// fn increment_file(filename: &str, increment: JournalIncrement, format: &str) -> Result<String> {
//     let current_as_datetime = file_to_datetime(filename, format);
//
//     let next_as_datetime = match increment {
//         JournalIncrement::Hours(hours) => current_as_datetime.checked_add_signed(TimeDelta::hours(hours)),
//         JournalIncrement::Days(days) => current_as_datetime.checked_add_signed(TimeDelta::days(days)),
//         JournalIncrement::Weeks(weeks) => current_as_datetime.checked_add_signed(TimeDelta::weeks(weeks)),
//         JournalIncrement::Months(months) => current_as_datetime.checked_add_months(months),
//         JournalIncrement::Years(years) => current_as_datetime.checked_add_months(12*years),
//     }
//
//     datetime_to_file(next_as_datetime, format)
// }

pub fn jump(settings: &Settings, jump_to: Option<&str>) -> Option<ShowDocumentParams> {
    // if jump_to is None, use the current time.
    // TODO: special syntax to reference the current file and the current time
    // TODO: make fuzzydate relative to any date
    // TODO: create file

    let note_file = match jump_to {
        Some(jmp_str) => parse(jmp_str).ok().and_then(|dt| datetime_to_file(dt, &settings.dailynote)),
        None => datetime_to_file(Local::now().naive_local(), &settings.dailynote)
    };

    note_file.map(|uri| ShowDocumentParams{ uri, 
        external: Some(false),
        take_focus: Some(true),
        selection: None })
}


// TODO; next and prev
