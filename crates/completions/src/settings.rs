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
}
