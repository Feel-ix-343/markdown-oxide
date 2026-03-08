use once_cell::sync::Lazy;
use regex::Regex;
use serde::de;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct MDMetadata {
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_tags", alias = "tag")]
    tags: Vec<String>,
}

/// Custom deserializer for frontmatter tags that handles:
/// - `tags: foo` (single string)
/// - `tags: foo, bar` (comma-separated string)
/// - `tags: [foo, bar]` (YAML inline list)
/// - `tags:\n  - foo\n  - bar` (YAML block list)
fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct TagsVisitor;

    impl<'de> de::Visitor<'de> for TagsVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or list of strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut tags = Vec::new();
            while let Some(tag) = seq.next_element::<String>()? {
                let trimmed = tag.trim().to_string();
                if !trimmed.is_empty() {
                    tags.push(trimmed);
                }
            }
            Ok(tags)
        }
    }

    deserializer.deserialize_any(TagsVisitor)
}

impl MDMetadata {
    pub fn new(text: &str) -> Option<MDMetadata> {
        // find text between --- at the beginning of the file

        static RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^---\n(?<metadata>(\n|.)*?)\n---").unwrap());

        let metadata_match = RE.captures_iter(text).next()?.name("metadata");

        let metadata_match = metadata_match?;

        let md_metadata = serde_yaml::from_str::<MDMetadata>(metadata_match.as_str());

        md_metadata.ok()
    }

    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
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
    fn test_tags_single_string() {
        let metadata = MDMetadata::new("---\ntags: foo\n---").unwrap();
        assert_eq!(metadata.tags(), &["foo"]);
    }

    #[test]
    fn test_tags_comma_separated() {
        let metadata = MDMetadata::new("---\ntags: foo, bar, baz\n---").unwrap();
        assert_eq!(metadata.tags(), &["foo", "bar", "baz"]);
    }

    #[test]
    fn test_tags_inline_list() {
        let metadata = MDMetadata::new("---\ntags: [foo, bar]\n---").unwrap();
        assert_eq!(metadata.tags(), &["foo", "bar"]);
    }

    #[test]
    fn test_tags_block_list() {
        let metadata = MDMetadata::new(
            r"---
tags:
  - foo
  - bar
---",
        )
        .unwrap();
        assert_eq!(metadata.tags(), &["foo", "bar"]);
    }

    #[test]
    fn test_tag_singular_alias() {
        let metadata = MDMetadata::new("---\ntag: mytag\n---").unwrap();
        assert_eq!(metadata.tags(), &["mytag"]);
    }

    #[test]
    fn test_tags_empty_list() {
        let metadata = MDMetadata::new("---\ntags: []\n---").unwrap();
        assert!(metadata.tags().is_empty());
    }

    #[test]
    fn test_tags_with_aliases() {
        let metadata = MDMetadata::new("---\naliases: [a1]\ntags: [t1, t2]\n---").unwrap();
        assert_eq!(metadata.aliases(), &["a1"]);
        assert_eq!(metadata.tags(), &["t1", "t2"]);
    }

    #[test]
    fn test_no_tags_field() {
        let metadata = MDMetadata::new("---\naliases: [a1]\n---").unwrap();
        assert!(metadata.tags().is_empty());
    }
}
