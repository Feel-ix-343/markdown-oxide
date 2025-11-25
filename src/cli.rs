use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};
use chrono::offset::Local;
use chrono::{Days, NaiveDate};
use clap::{Parser, Subcommand};
use fuzzydate::parse;

use crate::config::Settings;

#[derive(Parser)]
#[command(name = "markdown-oxide")]
#[command(author, version, about = "A PKM LSP for markdown files", long_about = "A PKM LSP for markdown files.\n\nWhen run without a command, starts the LSP server for use with text editors.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Open a daily note in your editor
    Daily {
        /// Date specification (e.g., "today", "yesterday", "tomorrow", "next monday", "+1", "-1")
        #[arg(default_value = "today")]
        date: Vec<String>,
    },
    /// Open the configuration file in your editor
    Config,
}

fn get_editor() -> String {
    std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        })
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

fn date_to_path(date: NaiveDate, dailynote_format: &str, daily_notes_folder: &Path) -> PathBuf {
    let filename = date.format(dailynote_format).to_string();
    daily_notes_folder.join(filename).with_extension("md")
}

fn resolve_date(date_str: &str) -> Result<NaiveDate> {
    let today = Local::now().date_naive();

    if let Some(offset) = parse_relative_directive(date_str) {
        let target_date = if offset >= 0 {
            today.checked_add_days(Days::new(offset as u64))
        } else {
            today.checked_sub_days(Days::new((-offset) as u64))
        };
        target_date.ok_or_else(|| anyhow!("Invalid date offset: {}", date_str))
    } else {
        parse(date_str)
            .map(|dt| dt.date())
            .map_err(|_| anyhow!("Could not parse date: {}", date_str))
    }
}

pub fn run_daily(date_args: Vec<String>, root_dir: &Path) -> Result<()> {
    let settings = Settings::new(root_dir, &Default::default())?;
    let daily_note_format = &settings.dailynote;
    let daily_notes_folder = root_dir.join(&settings.daily_notes_folder);

    let date_str = if date_args.is_empty() {
        "today".to_string()
    } else {
        date_args.join(" ")
    };

    let date = resolve_date(&date_str)?;
    let path = date_to_path(date, daily_note_format, &daily_notes_folder);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if !path.exists() {
        File::create(&path)?;
    }

    let editor = get_editor();
    let status = Command::new(&editor).arg(&path).status()?;

    if !status.success() {
        return Err(anyhow!("Editor '{}' exited with error", editor));
    }

    Ok(())
}

pub fn run_config(root_dir: &Path) -> Result<()> {
    let local_config = root_dir.join(".moxide.toml");
    let global_config = shellexpand::tilde("~/.config/moxide/settings.toml").to_string();
    let global_config_path = PathBuf::from(&global_config);

    let config_path = if local_config.exists() {
        local_config
    } else if global_config_path.exists() {
        global_config_path
    } else {
        if let Some(parent) = global_config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        File::create(&global_config_path)?;
        global_config_path
    };

    let editor = get_editor();
    let status = Command::new(&editor).arg(&config_path).status()?;

    if !status.success() {
        return Err(anyhow!("Editor '{}' exited with error", editor));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_parse_relative_directive() {
        assert_eq!(parse_relative_directive("prev"), Some(-1));
        assert_eq!(parse_relative_directive("next"), Some(1));
        assert_eq!(parse_relative_directive("+1"), Some(1));
        assert_eq!(parse_relative_directive("-1"), Some(-1));
        assert_eq!(parse_relative_directive("+7"), Some(7));
        assert_eq!(parse_relative_directive("-7"), Some(-7));
        assert_eq!(parse_relative_directive("invalid"), None);
    }

    #[test]
    fn test_resolve_date_today() {
        let today = Local::now().date_naive();
        assert_eq!(resolve_date("today").unwrap(), today);
    }

    #[test]
    fn test_resolve_date_relative() {
        let today = Local::now().date_naive();
        let tomorrow = today.checked_add_days(Days::new(1)).unwrap();
        let yesterday = today.checked_sub_days(Days::new(1)).unwrap();

        assert_eq!(resolve_date("+1").unwrap(), tomorrow);
        assert_eq!(resolve_date("-1").unwrap(), yesterday);
        assert_eq!(resolve_date("next").unwrap(), tomorrow);
        assert_eq!(resolve_date("prev").unwrap(), yesterday);
    }

    #[test]
    fn test_date_to_path() {
        let date = NaiveDate::from_ymd_opt(2024, 11, 25).unwrap();
        let path = date_to_path(date, "%Y-%m-%d", Path::new("/notes"));
        assert_eq!(path, PathBuf::from("/notes/2024-11-25.md"));
    }
}
