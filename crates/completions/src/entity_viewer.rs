use vault::{Referenceable, Vault};

use crate::entity::Entity;
use itertools::Itertools;

pub struct EntityViewer<'a>(&'a Vault);

impl EntityViewer<'_> {
    pub fn new(vault: &Vault) -> EntityViewer {
        EntityViewer(vault)
    }

    pub(crate) fn unindexed_block_entity_view(&self, it: &vault::Block) -> Option<String> {
        self.0
            .select_line_str(it.file, it.range.start.line as usize)
            .map(|it| it.to_string())
    }
}

// TODO implement CompletionResolve

impl EntityViewer<'_> {
    pub fn entity_view(&self, referenceable: &Referenceable) -> Option<String> {
        ui::referenceable_string(self.0, &[referenceable.clone()])
    }
}
