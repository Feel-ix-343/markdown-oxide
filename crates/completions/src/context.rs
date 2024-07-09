use moxide_config::Settings;
use std::fmt::Debug;
use vault::Vault;

use crate::{
    cmd_displayer::CmdDisplayer, entity_viewer::EntityViewer, parser::Parser, querier::Querier,
    settings::SettingsAdapter,
};

pub struct QueryContext<'fs: 'cache, 'cache> {
    // parser: Parser<'a>,
    vault: &'fs Vault,
    // querier: Querier<'a>,
    settings: SettingsAdapter<'fs>,
    entity_viewer: EntityViewer<'fs>,
    cmd_displayer: CmdDisplayer<'fs>,
    cache: &'cache mut crate::cache::QueryCache,
}

impl<'fs, 'cache> QueryContext<'fs, 'cache> {
    pub fn new(
        vault: &'fs Vault,
        settings: &'fs Settings,
        cache: &'cache mut crate::cache::QueryCache,
    ) -> Self {
        Self {
            vault,
            settings: SettingsAdapter::new(settings, vault),
            entity_viewer: EntityViewer::new(vault),
            cmd_displayer: CmdDisplayer::new(vault),
            cache,
        }
    }

    pub(crate) fn parser(&self) -> Parser<'fs> {
        Parser::new(self.vault)
    }
    pub(crate) fn querier(&'fs self) -> Querier<'fs> {
        Querier::new(self.vault)
    }
    pub(crate) fn settings(&self) -> &SettingsAdapter {
        &self.settings
    }
    pub(crate) fn entity_viewer(&self) -> &EntityViewer {
        &self.entity_viewer
    }

    pub(crate) fn cmd_displayer(&self) -> &CmdDisplayer<'fs> {
        &self.cmd_displayer
    }

    pub(crate) fn cache(&mut self) -> &mut crate::cache::QueryCache {
        self.cache
    }
}

impl Debug for QueryContext<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueryContext")
    }
}
