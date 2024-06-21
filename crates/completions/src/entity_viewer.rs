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
    pub fn entity_view(&self, named_entity: &Entity) -> Option<String> {
        let referenceable: Referenceable = named_entity.to_referenceable();

        ui::referenceable_string(self.0, &[referenceable])
    }
}
