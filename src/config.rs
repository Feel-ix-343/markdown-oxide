use std::path::Path;

use anyhow::anyhow;
use config::{Config, File};
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    /// Format of daily notes
    pub dailynote: String,
}

impl Settings {
    pub fn new(root_dir: &Path) -> anyhow::Result<Settings> {
        let obsidian_daily_note = obsidian_dailynote_converted(root_dir);

        let settings = Config::builder()
            .add_source(
                File::with_name(&format!(
                    "{}/.moxide",
                    root_dir
                        .to_str()
                        .ok_or(anyhow!("Can't convert root_dir to str"))?
                ))
                .required(false),
            )
            .add_source(File::with_name("~/.config/moxide/settings").required(false))
            .set_default(
                "dailynote",
                obsidian_daily_note.unwrap_or("%Y-%m-%d".to_string()),
            )
            .map_err(|err| anyhow!("Failed to parse due to: {err}"))?
            .build()
            .map_err(|err| anyhow!("Build err: {err}"))?;

        let settings = settings.try_deserialize::<Settings>()?;

        anyhow::Ok(settings)
    }
}

fn obsidian_dailynote_converted(root_dir: &Path) -> Option<String> {
    let daily_notes_config_file = root_dir.join(".obsidian").join("daily-notes.json");
    let file = std::fs::read(daily_notes_config_file).ok();
    let config: Option<HashMap<String, String>> =
        file.and_then(|file| serde_json::from_slice(&file).ok());

    let daily_note = config.as_ref().and_then(|config| {
        config
            .get("format")
            .map(|format| convert_momentjs_to_chrono_format(format))
    });

    daily_note
}

use std::collections::HashMap;

// GPT-4 code
fn momentjs_to_chrono_format_map() -> IndexMap<&'static str, &'static str> {
    let mut map = IndexMap::new();

    // Year
    map.insert("YYYY", "%Y");
    map.insert("YY", "%y");

    // Month
    map.insert("MMMM", "%B");
    map.insert("MMM", "%b");
    map.insert("MM", "%m");
    map.insert("M", "%-m");

    // Day
    map.insert("DD", "%d");
    map.insert("D", "%-d");

    // Weekday
    map.insert("dddd", "%A");
    map.insert("ddd", "%a");

    map
}

// GPT-4 code
fn convert_momentjs_to_chrono_format(moment_format: &str) -> String {
    let format_map = momentjs_to_chrono_format_map();
    let mut chrono_format = moment_format.to_string();
    println!("{:?}", chrono_format);

    for (moment_token, chrono_token) in format_map.iter() {
        println!("{:?}", chrono_token);
        chrono_format = chrono_format.replace(moment_token, chrono_token);
        println!("{:?}", chrono_format);
    }

    chrono_format
}

#[cfg(test)]
mod test {

    use crate::config::convert_momentjs_to_chrono_format;

    #[test]
    fn test_format_conversion() {
        let moment_format = "YYYY-MM-DD";
        let chrono_format = convert_momentjs_to_chrono_format(moment_format);
        assert_eq!(chrono_format, "%Y-%m-%d");
    }
}
