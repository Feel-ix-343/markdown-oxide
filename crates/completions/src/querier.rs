use std::ops::Deref;

use crate::{
    entity::{Block, Entity, NamedEntityTypeInfo::*},
    parser::{BlockQuery, EntityInfileQuery, EntityQuery},
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
        link_query: EntityQuery,
    ) -> impl IndexedParallelIterator<Item = Entity> {
        let named_entities = self.get_named_entities();
        let matchables = named_entities.map(MatchableNamedEntity::from);

        let matched = fuzzy_match(
            &link_query_string(link_query),
            matchables.collect::<Vec<_>>().into_iter(),
        );

        matched.into_par_iter().map(|(it, _)| it.into())
    }

    fn get_named_entities(&self) -> impl ParallelIterator<Item = Entity> {
        self.vault
            .select_referenceable_nodes(None)
            .into_par_iter()
            .flat_map(Entity::from_referenceable)
    }

    pub fn unnamed_query(
        &self,
        link_query: BlockQuery,
    ) -> impl IndexedParallelIterator<Item = Block> {
        let unnamed = self
            .vault
            .select_blocks()
            .into_par_iter()
            .flat_map(|it| {
                Block::from_block(
                    it.text,
                    it.range.start.line as usize,
                    it.range.end.character as usize,
                    it.file,
                )
            })
            .map(MatchableBlock::from);

        let matched = fuzzy_match(
            link_query.grep_string,
            unnamed.collect::<Vec<_>>().into_iter(),
        );

        matched.into_par_iter().map(|(it, _)| it.into())
    }

    pub fn indexed_block_info(&self, block: &Block) -> Option<String> {
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

fn link_query_string(link_query: EntityQuery) -> String {
    match link_query {
        EntityQuery {
            file_query: file_ref,
            infile_query: None,
        } => file_ref.to_string(),
        EntityQuery {
            file_query: file_ref,
            infile_query: Some(EntityInfileQuery::Heading(heading_string)),
        } => format!("{file_ref}#{heading_string}"),
        EntityQuery {
            file_query: file_ref,
            infile_query: Some(EntityInfileQuery::Index(index)),
        } => format!("{file_ref}#^{index}"),
    }
}

struct MatchableNamedEntity<'a>(String, Entity<'a>);

impl<'a> From<Entity<'a>> for MatchableNamedEntity<'a> {
    fn from(value: Entity<'a>) -> Self {
        let file_ref = value.info.path.file_name().unwrap().to_str().unwrap();

        let match_string = match value.info.type_info {
            Heading(heading) => format!("{file_ref}#{heading}"),
            IndexedBlock(index) => format!("{file_ref}#^{index}"),
            _ => file_ref.to_string(),
        };

        MatchableNamedEntity(match_string, value)
    }
}

impl<'a> From<MatchableNamedEntity<'a>> for Entity<'a> {
    fn from(value: MatchableNamedEntity<'a>) -> Self {
        value.1
    }
}

impl<'a> Deref for MatchableNamedEntity<'a> {
    type Target = Entity<'a>;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl Matchable for MatchableNamedEntity<'_> {
    fn match_string(&self) -> &str {
        &self.0
    }
}

struct MatchableBlock<'a>(String, Block<'a>);

impl<'a> From<Block<'a>> for MatchableBlock<'a> {
    fn from(value: Block<'a>) -> Self {
        MatchableBlock(value.info.line_text.to_string(), value)
    }
}

impl<'a> From<MatchableBlock<'a>> for Block<'a> {
    fn from(value: MatchableBlock<'a>) -> Self {
        value.1
    }
}

impl Matchable for MatchableBlock<'_> {
    fn match_string(&self) -> &str {
        &self.0
    }
}

impl<'a> Deref for MatchableBlock<'a> {
    type Target = Block<'a>;
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
pub fn fuzzy_match<T>(
    filter_text: &str,
    items: impl Iterator<Item = T>,
) -> impl IndexedParallelIterator<Item = (T, u32)>
where
    T: Matchable + Send,
{
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
