use vault::{Referenceable, Vault};

use crate::entity::Entity;

pub struct EntityViewer<'a>(&'a Vault);

impl EntityViewer<'_> {
    pub fn new(vault: &Vault) -> EntityViewer {
        EntityViewer(vault)
    }
}

// TODO implement CompletionResolve

impl EntityViewer<'_> {
    pub fn entity_view(&self, referenceable: Referenceable) -> Option<String> {
        ui::referenceable_string(self.0, &[referenceable])
    }
}
