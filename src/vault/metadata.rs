use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize};

#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct MDMetadata {
    aliases: Vec<String>,
}

impl MDMetadata {
    pub fn new(text: &str) -> Option<MDMetadata> {
        // find text between --- at the beginning of the file

        static RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^---\n(?<metadata>(\n|.)*?)\n---").unwrap());

        let metadata_match = RE.captures_iter(text).next()?.name("metadata");

        println!("metadata_match: {:?}", metadata_match);

        let metadata_match = metadata_match?;

        let md_metadata = serde_yaml::from_str::<MDMetadata>(metadata_match.as_str());

        println!("md_metadata: {:?}", md_metadata);

        md_metadata.ok()
    }

    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
}

#[cfg(test)]
mod tests {
    use crate::vault::metadata::MDMetadata;

    #[test]
    fn test_aliases() {
        let metadata = MDMetadata::new("---\naliases: [\"alias1\", \"alias2\"]\n---").unwrap();
        assert_eq!(metadata.aliases, vec!["alias1", "alias2"]);
    }

    #[test]
    fn test_alias_list() {
        let metadata = MDMetadata::new(
            r"---
aliases:
    - alias1
    - alias2
---",
        )
        .unwrap();
        assert_eq!(metadata.aliases(), &["alias1", "alias2"]);
    }
}
