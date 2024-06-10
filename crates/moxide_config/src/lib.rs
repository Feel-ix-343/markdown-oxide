use std::path::Path;

use anyhow::anyhow;
use config::{Config, File};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;
use tower_lsp::lsp_types::ClientCapabilities;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    /// Format of daily notes
    pub dailynote: String,
    /// Diffrent pages path than default
    pub new_file_folder_path: String,
    pub daily_notes_folder: String,
    pub heading_completions: bool,
    pub title_headings: bool,
    pub unresolved_diagnostics: bool,
    pub semantic_tokens: bool,
    pub tags_in_codeblocks: bool,
    pub references_in_codeblocks: bool,
}

impl Settings {
    pub fn new(root_dir: &Path, capabilities: &ClientCapabilities) -> anyhow::Result<Settings> {
        let (obsidian_daily_note, obsidian_daily_notes_folder) =
            obsidian_dailynote_details(root_dir);
        let obsidian_new_file_folder_path = obsidian_new_file_folder_path(root_dir);
        let expanded = shellexpand::tilde("~/.config/moxide/settings");
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
            .add_source(File::with_name(&expanded).required(false))
            .set_default(
                "new_file_folder_path",
                obsidian_new_file_folder_path.unwrap_or("".to_string()),
            )?
            .set_default(
                "daily_notes_folder",
                obsidian_daily_notes_folder.unwrap_or("".to_string()),
            )?
            .set_default(
                "dailynote",
                obsidian_daily_note.unwrap_or("%Y-%m-%d".to_string()),
            )?
            .set_default("heading_completions", true)?
            .set_default("unresolved_diagnostics", true)?
            .set_default("title_headings", true)?
            .set_default("semantic_tokens", true)?
            .set_default("tags_in_codeblocks", true)?
            .set_default("references_in_codeblocks", true)?
            .set_override_option(
                "semantic_tokens",
                capabilities.text_document.as_ref().and_then(|it| {
                    match it.semantic_tokens.is_none() {
                        true => Some(false),
                        false => None,
                    }
                }),
            )?
            .build()
            .map_err(|err| anyhow!("Build err: {err}"))?;

        let settings = settings.try_deserialize::<Settings>()?;

        anyhow::Ok(settings)
    }
}

fn obsidian_dailynote_details(root_dir: &Path) -> (Option<String>, Option<String>) {
    let daily_notes_config_file = root_dir.join(".obsidian").join("daily-notes.json");
    let file = std::fs::read(daily_notes_config_file).ok();
    let config: Option<HashMap<String, String>> =
        file.and_then(|file| serde_json::from_slice(&file).ok());
    let daily_note = config.as_ref().and_then(|config| {
        config
            .get("format")
            .map(|format| convert_momentjs_to_chrono_format(format))
    });

    let daily_notes_folder = config
        .as_ref()
        .and_then(|config| config.get("folder").cloned());

    (daily_note, daily_notes_folder)
}

fn obsidian_new_file_folder_path(root_dir: &Path) -> Option<String> {
    let obsidian_settings_file = root_dir.join(".obsidian").join("app.json");
    let file = std::fs::read(obsidian_settings_file).ok();
    let config: Option<HashMap<String, Value>> = file.and_then(|file| {
        let parsed = serde_json::from_slice(&file);
        parsed.ok()
    });

    let new_file_folder_path = config.as_ref().and_then(|config| {
        let path = config
            .get("newFileFolderPath")
            .and_then(|value| value.as_str())
            .map(String::from);

        if config.get("newFileLocation").and_then(|v| v.as_str()) == Some("folder") {
            path
        } else {
            None
        }
    });

    new_file_folder_path
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

fn convert_momentjs_to_chrono_format(moment_format: &str) -> String {
    let format_map = momentjs_to_chrono_format_map();
    let mut chrono_format = moment_format.to_string();

    for (moment_token, chrono_token) in format_map.iter() {
        chrono_format = chrono_format.replace(moment_token, chrono_token);
    }

    chrono_format
}
