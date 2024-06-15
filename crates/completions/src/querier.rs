use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::{
    entity::NamedEntity,
    entity::NamedEntityTypeInfo::*,
    parser::{NamedEntityInfileQuery, NamedEntityQuery},
};
use nucleo_matcher::{
    pattern::{self, Normalization},
    Matcher,
};
use rayon::prelude::*;
use vault::{MDHeading, MDIndexedBlock, Referenceable, Vault};

pub(crate) struct Querier<'a> {
    vault: &'a Vault,
}

impl<'a> Querier<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}

impl<'a> Querier<'a> {
    pub fn query(
        &self,
        link_query: NamedEntityQuery,
    ) -> impl IndexedParallelIterator<Item = NamedEntity> {
        let named_entities = self.get_named_entities();
        let matchables = named_entities.map(MatchableNamedEntity::from);

        let matched = fuzzy_match(
            &link_query_string(link_query),
            matchables.collect::<Vec<_>>().into_iter(),
        );

        matched.into_par_iter().map(|(it, _)| it.into())
    }

    fn get_named_entities(&self) -> impl ParallelIterator<Item = NamedEntity<'a>> {
        self.vault
            .select_referenceable_nodes(None)
            .into_par_iter()
            .flat_map(|it| NamedEntity::from_referenceable(it))
    }
}

fn link_query_string(link_query: NamedEntityQuery) -> String {
    match link_query {
        NamedEntityQuery {
            file_query: file_ref,
            infile_query: None,
        } => file_ref.to_string(),
        NamedEntityQuery {
            file_query: file_ref,
            infile_query: Some(NamedEntityInfileQuery::Heading(heading_string)),
        } => format!("{file_ref}#{heading_string}"),
        NamedEntityQuery {
            file_query: file_ref,
            infile_query: Some(NamedEntityInfileQuery::Index(index)),
        } => format!("{file_ref}#^{index}"),
    }
}

struct MatchableNamedEntity<'a>(String, NamedEntity<'a>);

impl<'a> From<NamedEntity<'a>> for MatchableNamedEntity<'a> {
    fn from(value: NamedEntity<'a>) -> Self {
        let file_ref = value.info().path.file_name().unwrap().to_str().unwrap();

        let match_string = match value.info().type_info {
            Heading(heading) => format!("{file_ref}#{heading}"),
            IndexedBlock(index) => format!("{file_ref}#^{index}"),
            _ => file_ref.to_string(),
        };

        MatchableNamedEntity(match_string, value)
    }
}

impl<'a> From<MatchableNamedEntity<'a>> for NamedEntity<'a> {
    fn from(value: MatchableNamedEntity<'a>) -> Self {
        value.1
    }
}

impl<'a> Deref for MatchableNamedEntity<'a> {
    type Target = NamedEntity<'a>;
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
