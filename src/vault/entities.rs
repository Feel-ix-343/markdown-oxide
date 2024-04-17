use std::path::Path;

use rayon::iter::ParallelIterator;

use super::{md_nodes::{MDBlock, MDFile}, Entity, FileRange, Vault};


pub struct FileEntity<'handler, 'fs, R: FileReferencer<'fs>, T: Blocks<'handler, 'fs, R>> {
    md_file: &'handler T,
    vault: &'handler Vault,
}

impl<'handler, 'fs, R: FileReferencer<'fs>, T: Blocks<'handler, 'fs, R>> Entity<'fs> for FileEntity<'handler, 'fs, R, T> {
    fn view(&self) -> super::EntityView<'fs> {
        todo!()
    }

    fn incoming(&self) -> impl rayon::prelude::ParallelIterator<Item = super::Reference<'fs, super::IncomingReference<'fs>>> {
        todo!()
    }

    fn outgoing(&self) -> impl rayon::prelude::ParallelIterator<Item = super::Reference<'fs, super::OutgoingReference<'fs>>> {

        self.md_file.blocks()
            .map(|block| block.file_references()) // Item = Iter<References>
            .flatten() // Item = Reference
            .flat_map(|reference| {
                // Get optional file from vault
                let referenced_file = self.vault.parsed_file(reference.1)
            })

    }

    fn parent(&self) -> &'fs dyn Entity<'fs> {
        todo!()
    }

    fn children(&self) -> impl rayon::prelude::ParallelIterator<Item = &'fs dyn Entity<'fs>> {
        todo!()
    }
}


trait Blocks<'handler, 'fs, R: FileReferencer<'fs>> {
    fn blocks(&self) -> impl ParallelIterator<Item = R> + 'fs;
}

trait FileReferencer<'fs> {
    /// References to filename (not the full path)
    fn file_references(&self) -> &[(FileRange<'fs>, &'fs str)];
}
