use std::{path::Path, collections::HashMap};

use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use super::parsing::{Vault,  Match};

//pub mod graph;
//pub mod analyzer;
//pub mod markdownparser;
//pub mod nodes;

pub struct Analysis {
    /// Map of files by their ref name
    vault: Vault
}

impl Analysis {
    pub fn new(dir: &str) -> Result<Analysis, std::io::Error>  {
        let vault = Vault::new(dir)?;

        return Ok(Analysis {
            vault
        })
    }

    pub fn get_references() {
        // Get the linking node that the cursor is over through getting all linking nodes in the file and if not on any of them, return the file itself. 

        // Find links that reference this node
    }


    pub fn file_incoming(&self, file_ref: &str) -> Vec<&Match>  {
        let file_ref_string = String::from(file_ref);

        // Check all linking nodes in the vault for this file_ref. 
        return self.vault.get_linking_nodes().iter()
            .flat_map(|n| n.get_links())
            .filter(|l| l.link_ref == file_ref_string || l.link_ref.starts_with(&format!("{}#", file_ref_string)))
            .map(|l| &l.link_match)
            .collect_vec();
    }

    pub fn heading_incoming(&self, file_ref: &str, heading: &str) -> Vec<&Match> {
        let heading_ref = format!("{}#{}", file_ref, heading);
        return self.vault.get_linking_nodes().iter()
            .flat_map(|n| n.get_links())
            .filter(|l| l.link_ref == heading_ref)
            .map(|l| &l.link_match)
            .collect_vec();
    }

    pub fn tags_incoming(&self, tag: &str) -> Vec<&Match> {
        return self.vault.files.iter()
            .flat_map(|(_r, f)| &f.tags)
            .filter(|t| t.tag == String::from(tag) || t.tag.starts_with(&format!("{}/", tag)))
            .map(|t| &t.file_match)
            .collect_vec();
    }

    pub fn block_incoming(&self, file_ref: &str, block_index: &str) -> Vec<&Match> {
        let block_ref = format!("{}#^{}", file_ref, block_index);
        return self.vault.get_linking_nodes().iter()
            .flat_map(|n| n.get_links())
            .filter(|l| l.link_ref == block_ref)
            .map(|l| &l.link_match)
            .collect_vec()
    }
}
