use chrono::{NaiveDateTime, TimeDelta};
use chrono::format::{Parsed, parse, strftime::StrftimeItems};

enum JournalIncrement {
    Hours(i8),
    Days(i8),
    Weeks(i8),
    Months(i8),
    Years(i8)
}


fn file_to_datetime(filename: &str, format: &str) -> Result<NaiveDateTime> {
    // default to the beginning of an underspecified interval,
    // top of hour, beginning of day, start of week

    let items = StrftimeItems::new(format);
    let mut parsed = Parsed::new();
    parse(&mut parsed, "2024-04-01", items);
    parsed.to_naive_datetime_with_offset(0)
}


fn datetime_to_file(datetime: NaiveDateTime, format: &str) -> String {
    datetime.format(format).to_string()
}

fn increment_file(filename: &str, increment: JournalIncrement, format: &str) -> Result<String> {
    let current_as_datetime = file_to_datetime(filename, format);

    let next_as_datetime = match increment {
        JournalIncrement::Hours(hours) => current_as_datetime.checked_add_signed(TimeDelta::hours(hours)),
        JournalIncrement::Days(days) => current_as_datetime.checked_add_signed(TimeDelta::days(days)),
        JournalIncrement::Weeks(weeks) => current_as_datetime.checked_add_signed(TimeDelta::weeks(weeks)),
        JournalIncrement::Months(months) => current_as_datetime.checked_add_months(months),
        JournalIncrement::Years(years) => current_as_datetime.checked_add_months(12*years),
    }

    datetime_to_file(next_as_datetime, format)
}

fn now() {
    // open the journal entry for the current moment
}

fn next(current_file: &str, increment: &str) {
    // TODO: use into JournalIncrement
    // open the journal entry for some number of intervals in the future
}

fn prev(current_file: &str, increment: &str) {
    // TODO: use into JournalIncrement
    // open the journal entry for some number of intervals in the past
}

fn jump(jump_to: &str) {
    // go to some note (next monday, 12/23/2024)
}
