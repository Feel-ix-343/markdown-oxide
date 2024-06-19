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
        vec![].into_par_iter()
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

struct MatchableNamedEntity<'a>(String, Entity<'a, NamedEntityData<'a>>);

impl<'a> From<Entity<'a, NamedEntityData<'a>>> for MatchableNamedEntity<'a> {
    fn from(value: Entity<'a, NamedEntityData<'a>>) -> Self {
        let file_ref = value.info().path.file_name().unwrap().to_str().unwrap();

        let match_string = match value.info().type_info {
            Heading(heading) => format!("{file_ref}#{heading}"),
            IndexedBlock(index) => format!("{file_ref}#^{index}"),
            _ => file_ref.to_string(),
        };

        MatchableNamedEntity(match_string, value)
    }
}

impl<'a> From<MatchableNamedEntity<'a>> for Entity<'a, NamedEntityData<'a>> {
    fn from(value: MatchableNamedEntity<'a>) -> Self {
        value.1
    }
}

impl<'a> Deref for MatchableNamedEntity<'a> {
    type Target = Entity<'a, NamedEntityData<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl Matchable for MatchableNamedEntity<'_> {
    fn match_string(&self) -> &str {
        &self.0
    }
}

impl<'a> Matchable for (String, &'a PathBuf) {
    fn match_string(&self) -> &str {
        self.0.as_str()
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
