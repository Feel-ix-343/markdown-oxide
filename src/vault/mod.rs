use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Position, Range, Url};
use walkdir::WalkDir;

use self::{file_metadata::FileMetadata, heading_to_slug::heading_to_slug};

pub mod file_metadata;
pub mod heading_to_slug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Referenceable<'a> {
    File(&'a Path, &'a MDFile),
    Heading(&'a Path, &'a MDHeading),
    Tag(&'a Path, &'a MDTag),
    Footnote(&'a Path, &'a MDFootnote),
    IndexedBlock(&'a Path, &'a IndexedBlock),
    UnresolvedHeading(&'a Path, &'a str),
    UnresolvedFile(&'a str),
    UnresolvedTag(&'a str),
    Alias(&'a Path, &'a str, &'a str), // path, alias name, original refname
}

impl<'a> Referenceable<'a> {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Referenceable::File(path, _) => Some(path),
            Referenceable::Heading(path, _) => Some(path),
            Referenceable::Tag(path, _) => Some(path),
            Referenceable::Footnote(path, _) => Some(path),
            Referenceable::IndexedBlock(path, _) => Some(path),
            Referenceable::Alias(path, _, _) => Some(path),
            Referenceable::UnresolvedHeading(_, _) => None,
            Referenceable::UnresolvedFile(_) => None,
            Referenceable::UnresolvedTag(_) => None,
        }
    }

    pub fn refname(&self) -> &str {
        match self {
            Referenceable::File(_, file) => &file.name,
            Referenceable::Heading(_, heading) => &heading.slug,
            Referenceable::Tag(_, tag) => &tag.tag_ref,
            Referenceable::Footnote(_, footnote) => &footnote.index,
            Referenceable::IndexedBlock(_, block) => &block.refname,
            Referenceable::Alias(_, alias, _) => alias,
            Referenceable::UnresolvedHeading(_, heading) => heading,
            Referenceable::UnresolvedFile(file) => file,
            Referenceable::UnresolvedTag(tag) => tag,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        matches!(
            self,
            Referenceable::UnresolvedHeading(..)
                | Referenceable::UnresolvedFile(..)
                | Referenceable::UnresolvedTag(..)
        )
    }
}

#[derive(Debug, Clone)]
pub struct Vault {
    pub root_dir: PathBuf,
    pub md_files: HashMap<PathBuf, MDFile>,
    pub file_metadata: FileMetadata,
}

impl Vault {
    pub fn select_referenceable_nodes<'a>(
        &'a self,
        path: Option<&Path>,
    ) -> Vec<Referenceable<'a>> {
        let mut referenceables = Vec::new();

        // Add files
        if path.is_none() {
            referenceables.extend(
                self.md_files
                    .iter()
                    .map(|(path, file)| Referenceable::File(path.as_path(), file)),
            );
        }

        // Add headings, tags, footnotes, blocks
        for (file_path, file) in &self.md_files {
            if let Some(path) = path {
                if file_path != path {
                    continue;
                }
            }

            // Add headings
            referenceables.extend(
                file.headings
                    .iter()
                    .map(|heading| Referenceable::Heading(file_path.as_path(), heading)),
            );

            // Add tags
            referenceables.extend(
                file.tags.iter()
                    .map(|tag| Referenceable::Tag(file_path.as_path(), tag)),
            );

            // Add footnotes
            referenceables.extend(
                file.footnotes
                    .iter()
                    .map(|footnote| Referenceable::Footnote(file_path.as_path(), footnote)),
            );

            // Add blocks
            referenceables.extend(
                file.blocks
                    .iter()
                    .map(|block| Referenceable::IndexedBlock(file_path.as_path(), block)),
            );

            // Add aliases from frontmatter
            if let Some(aliases) = &file.metadata.aliases {
                referenceables.extend(aliases.iter().map(|alias| {
                    Referenceable::Alias(file_path.as_path(), alias, &file.name)
                }));
            }
        }

        referenceables
    }
}