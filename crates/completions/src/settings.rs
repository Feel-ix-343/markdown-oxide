use std::path::{Path, PathBuf};

use moxide_config::Settings;
use vault::Vault;

#[derive(Debug, Clone, Copy)]
pub struct SettingsAdapter<'a>(&'a Settings, &'a Vault);

impl SettingsAdapter<'_> {
    pub fn new<'a>(settings: &'a Settings, vault: &'a Vault) -> SettingsAdapter<'a> {
        SettingsAdapter(settings, vault)
    }
}

pub enum DailyNoteDisplay {
    WikiAndMD,
    Wiki,
    MD,
    Neither,
}

impl SettingsAdapter<'_> {
    pub fn include_md_extension(&self) -> bool {
        // TODO
        false
    }

    pub fn num_completions(&self) -> usize {
        // TODO
        50
    }

    pub fn block_compeltions_display_text(&self) -> bool {
        // TODO
        true
    }

    pub fn daily_note_format(&self) -> &str {
        &self.0.dailynote
    }

    pub fn alias_display_text(&self) -> bool {
        // TODO
        true
    }

    pub fn completion_preselect(&self) -> bool {
        // TODO
        true
    }

    pub(crate) fn daily_note_folder_path(&self) -> &Path {
        // TODO
        self.1.root_dir()
    }

    pub(crate) fn daily_note_display_text(&self) -> DailyNoteDisplay {
        // TODO
        DailyNoteDisplay::MD
    }

    pub(crate) fn num_block_completions(&self) -> usize {
        // TODO
        20
    }
}
