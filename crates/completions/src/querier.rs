use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::{
    entity::{Entity, NamedEntityData, NamedEntityTypeInfo::*, UnnamedEntityData},
    parser::{EntityQuery, NamedEntityInfileQuery, NamedQueryData, UnnamedQueryData},
};
use nucleo_matcher::{
    pattern::{self, Normalization},
    Matcher,
};
use rayon::prelude::*;
use vault::Vault;

pub(crate) struct Querier<'a> {
    vault: &'a Vault,
}

impl<'a> Querier<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}

impl<'a> Querier<'a> {
    pub fn named_query(
        &self,
        link_query: EntityQuery<NamedQueryData>,
    ) -> impl IndexedParallelIterator<Item = Entity<NamedEntityData>> {
        let named_entities = self.get_named_entities();
        let matchables = named_entities.map(MatchableNamedEntity::from);

        let matched = fuzzy_match(
            &link_query_string(link_query),
            matchables.collect::<Vec<_>>().into_iter(),
        );

        matched.into_par_iter().map(|(it, _)| it.into())
    }

    fn get_named_entities(&self) -> impl ParallelIterator<Item = Entity<NamedEntityData>> {
        self.vault
            .select_referenceable_nodes(None)
            .into_par_iter()
            .flat_map(|it| Entity::from_referenceable(it))
    }

    pub fn unnamed_query(
        &self,
        link_query: EntityQuery<UnnamedQueryData>,
    ) -> impl IndexedParallelIterator<Item = Entity<UnnamedEntityData>> {
        let unnamed = self
            .vault
            .select_blocks()
            .into_par_iter()
            .flat_map(|it| {
                Entity::from_block(
                    it.text,
                    it.range.start.line as usize,
                    it.range.end.character as usize,
                    it.file,
                )
            })
            .map(MatchableUnnamedEntity::from);

        let matched = fuzzy_match(
            link_query.data.grep_string,
            unnamed.collect::<Vec<_>>().into_iter(),
        );

        matched.into_par_iter().map(|(it, _)| it.into())
    }

    pub fn indexed_block_info(&self, block: &Entity<UnnamedEntityData>) -> Option<String> {
        self.vault
            .select_referenceable_nodes(Some(block.location_info().2))
            .into_par_iter()
            .find_map_any(|it| match it {
                vault::Referenceable::IndexedBlock(_, indexed)
                    if indexed.range.start.line as usize == block.location_info().0 =>
                {
                    Some(indexed.index.clone())
                }
                _ => None,
            })
    }
}

fn link_query_string(link_query: EntityQuery<NamedQueryData>) -> String {
    match link_query.data {
        NamedQueryData {
            file_query: file_ref,
            infile_query: None,
        } => file_ref.to_string(),
        NamedQueryData {
            file_query: file_ref,
            infile_query: Some(NamedEntityInfileQuery::Heading(heading_string)),
        } => format!("{file_ref}#{heading_string}"),
        NamedQueryData {
            file_query: file_ref,
            infile_query: Some(NamedEntityInfileQuery::Index(index)),
        } => format!("{file_ref}#^{index}"),
    }
}

struct MatchableNamedEntity<'a>(String, Entity<NamedEntityData<'a>>);

impl<'a> From<Entity<NamedEntityData<'a>>> for MatchableNamedEntity<'a> {
    fn from(value: Entity<NamedEntityData<'a>>) -> Self {
        let file_ref = value.info().path.file_name().unwrap().to_str().unwrap();

        let match_string = match value.info().type_info {
            Heading(heading) => format!("{file_ref}#{heading}"),
            IndexedBlock(index) => format!("{file_ref}#^{index}"),
            _ => file_ref.to_string(),
        };

        MatchableNamedEntity(match_string, value)
    }
}

impl<'a> From<MatchableNamedEntity<'a>> for Entity<NamedEntityData<'a>> {
    fn from(value: MatchableNamedEntity<'a>) -> Self {
        value.1
    }
}

impl<'a> Deref for MatchableNamedEntity<'a> {
    type Target = Entity<NamedEntityData<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl Matchable for MatchableNamedEntity<'_> {
    fn match_string(&self) -> &str {
        &self.0
    }
}

struct MatchableUnnamedEntity<'a>(String, Entity<UnnamedEntityData<'a>>);

impl<'a> From<Entity<UnnamedEntityData<'a>>> for MatchableUnnamedEntity<'a> {
    fn from(value: Entity<UnnamedEntityData<'a>>) -> Self {
        MatchableUnnamedEntity(value.info().line_text.to_string(), value)
    }
}

impl<'a> From<MatchableUnnamedEntity<'a>> for Entity<UnnamedEntityData<'a>> {
    fn from(value: MatchableUnnamedEntity<'a>) -> Self {
        value.1
    }
}

impl Matchable for MatchableUnnamedEntity<'_> {
    fn match_string(&self) -> &str {
        &self.0
    }
}

impl<'a> Deref for MatchableUnnamedEntity<'a> {
    type Target = Entity<UnnamedEntityData<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

pub trait Matchable {
    fn match_string(&self) -> &str;
}

struct NucleoMatchable<T: Matchable>(T);
impl<T: Matchable> Deref for NucleoMatchable<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Matchable> AsRef<str> for NucleoMatchable<T> {
    fn as_ref(&self) -> &str {
        self.match_string()
    }
}

// TODO: parallelize this
pub fn fuzzy_match<'a, T: Matchable + Send>(
    filter_text: &str,
    items: impl Iterator<Item = T>,
) -> impl IndexedParallelIterator<Item = (T, u32)> {
    let items = items.map(NucleoMatchable);

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);

    let matches = pattern::Pattern::parse(
        filter_text,
        pattern::CaseMatching::Smart,
        Normalization::Smart,
    )
    .match_list(items, &mut matcher);

    matches.into_par_iter().map(|(item, score)| (item.0, score))
}
