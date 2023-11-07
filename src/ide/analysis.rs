use std::{path::Path, collections::HashMap};

use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use super::parsing::{Vault, get_parsed_vault, Match};

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
        let vault = get_parsed_vault(dir)?;

        return Ok(Analysis {
            vault
        })
    }

    /// Find all of the paragraphs and headings that have this file_ref as an outgoing link. Return a match to these paragraphs and headings
    pub fn file_incoming(&self, file_ref: &str) -> Option<Vec<&Match>>  {
        let file_ref_string = String::from(file_ref);

        let paragraph_matches = self.vault.files.iter()
            .flat_map(|(r, f)| &f.paragraphs)
            .filter(|p| p.resolved_links.iter().map(|l| &l.link_ref).any(|l| l == &file_ref_string || l.starts_with(&format!("{}#", file_ref_string))))
            .map(|p| &p.file_match)
            .collect_vec();

        let heading_matches = self.vault.files.iter()
            .flat_map(|(r, f)| &f.headings)
            .filter(|h| h.resolved_links.iter().map(|l| &l.link_ref).any(|l| l == &file_ref_string || l.starts_with(&format!("{}#", file_ref_string))))
            .map(|p| &p.file_match)
            .collect_vec();

        let matches = paragraph_matches.into_iter().chain(heading_matches.into_iter()).collect_vec();
        if matches.is_empty() {
            return None;
        } else {
            return Some(matches)
        }
    }

    pub fn heading_incoming(&self, file_ref: &str, heading: &str) -> Option<Vec<&Match>> {
        let heading_ref = format!("{}#{}", file_ref, heading);
        let paragraph_matches = self.vault.files.iter()
            .flat_map(|(r, f)| &f.paragraphs)
            .filter(|p| p.resolved_links.iter().map(|l| &l.link_ref).contains(&heading_ref))
            .map(|p| &p.file_match)
            .collect_vec();

        let heading_matches = self.vault.files.iter()
            .flat_map(|(r, f)| &f.headings)
            .filter(|h| h.resolved_links.iter().map(|l| &l.link_ref).contains(&heading_ref))
            .map(|p| &p.file_match)
            .collect_vec();

        let matches = paragraph_matches.into_iter().chain(heading_matches.into_iter()).collect_vec();
        if matches.is_empty() {
            return None;
        } else {
            return Some(matches)
        }
    }

    pub fn tags_incoming(&self, tag: &str) -> Option<Vec<&Match>> {
        let tags = self.vault.files.iter()
            .flat_map(|(_r, f)| &f.tags)
            .filter(|t| t.tag == String::from(tag) || t.tag.starts_with(&format!("{}/", tag)))
            .map(|t| &t.file_match)
            .collect_vec();

        if tags.is_empty() {
            return None
        } else {
            return Some(tags)
        }

    }
}
