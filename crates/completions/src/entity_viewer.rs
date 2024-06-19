use vault::{Preview, Referenceable, Vault};

use crate::entity::{Entity, NamedEntityData};

pub struct EntityViewer<'a>(&'a Vault);

impl EntityViewer<'_> {
    pub fn new(vault: &Vault) -> EntityViewer {
        EntityViewer(vault)
    }
}

// TODO implement CompletionResolve

impl EntityViewer<'_> {
    pub fn entity_view(&self, named_entity: &Entity<NamedEntityData>) -> Option<String> {
        let referenceable: Referenceable = named_entity.into();

        ui::referenceable_string(self.0, &[referenceable])
    }
}
