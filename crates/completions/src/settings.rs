use moxide_config::Settings;

pub struct SettingsAdapter<'a>(&'a Settings);

impl SettingsAdapter<'_> {
    pub fn new(settings: &Settings) -> SettingsAdapter {
        SettingsAdapter(settings)
    }
}

impl SettingsAdapter<'_> {
    pub fn include_md_extension(&self) -> bool {
        // TODO
        false
    }

    pub fn num_completions(&self) -> usize {
        // TODO
        10
    }

    pub fn block_compeltions_display_text(&self) -> bool {
        // TODO
        false
    }
}
