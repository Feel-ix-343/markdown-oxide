use crate::{
    cmd_displayer::CmdDisplayer, entity_viewer::EntityViewer, parser::Parser, querier::Querier,
    settings::SettingsAdapter,
};

pub struct Context<'a> {
    parser: Parser<'a>,
    querier: Querier<'a>,
    settings: SettingsAdapter<'a>,
    entity_viewer: EntityViewer<'a>,
    cmd_displayer: CmdDisplayer<'a>,
}

impl<'a> Context<'a> {
    pub fn new(
        parser: Parser<'a>,
        querier: Querier<'a>,
        settings: SettingsAdapter<'a>,
        entity_viewer: EntityViewer<'a>,
        cmd_displayer: CmdDisplayer<'a>,
    ) -> Self {
        Self {
            parser,
            querier,
            settings,
            entity_viewer,
            cmd_displayer,
        }
    }

    pub fn parser(&self) -> &Parser {
        &self.parser
    }
    pub fn querier(&self) -> &Querier {
        &self.querier
    }
    pub fn settings(&self) -> &SettingsAdapter {
        &self.settings
    }
    pub fn entity_viewer(&self) -> &EntityViewer {
        &self.entity_viewer
    }

    pub fn cmd_displayer(&self) -> &CmdDisplayer<'a> {
        &self.cmd_displayer
    }
}
