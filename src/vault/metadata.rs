use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct MDMetadata {
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    comments: Vec<String>,
}

impl MDMetadata {
    pub fn new(text: &str) -> Option<MDMetadata> {
        // find text between --- at the beginning of the file

        static RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^---\n(?<metadata>(\n|.)*?)\n---").unwrap());

        static COMMENT_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?m)(?:^[ \t]*#|\s#)\s*(?P<comment>[^\r\n]*)(?:\r?\n|$)").unwrap()
        });

        let metadata_match = RE.captures_iter(text).next()?.name("metadata");

        let metadata_match = metadata_match?;

        let metadata_str = metadata_match.as_str();

        let md_metadata = serde_yaml::from_str::<MDMetadata>(metadata_str);

        let comment_match = COMMENT_RE
            .captures_iter(metadata_str)
            .filter_map(|c| c.name("comment"))
            .map(|i| i.as_str().into())
            .collect::<Vec<_>>();

        match md_metadata {
            Ok(md) if md.aliases.is_empty() && comment_match.is_empty() => None,
            Ok(mut md) => {
                md.comments = comment_match;
                Some(md)
            }
            Err(_) => None,
        }
    }

    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
    pub fn comments(&self) -> &[String] {
        &self.comments
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

    #[test]
    fn test_alias_with_comments() {
        let metadata = MDMetadata::new(
            r"---
aliases:
    # Comment 1
    - alias1
    - alias2 # Comment 2
---",
        )
        .unwrap();
        assert_eq!(metadata.aliases(), vec!["alias1", "alias2"]);
        assert_eq!(metadata.comments(), vec!["Comment 1", "Comment 2"]);
    }
}
