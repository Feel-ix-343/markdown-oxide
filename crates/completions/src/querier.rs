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
        link_query: EntityQuery<'_>,
    ) -> impl IndexedParallelIterator<Item = Entity<'a>> + 'a {
        let named_entities = self.get_named_entities().collect::<Vec<_>>();

        let matched = run_query(link_query.to_query_string(), named_entities.into_iter());

        matched
    }

    fn get_named_entities(&self) -> impl ParallelIterator<Item = Entity<'a>> {
        self.vault
            .select_referenceable_nodes(None)
            .into_par_iter()
            .flat_map(Entity::from_referenceable)
    }

    pub fn unnamed_query(
        &self,
        link_query: BlockQuery<'_>,
    ) -> impl IndexedParallelIterator<Item = Block<'a>> {
        let blocks = self.vault.select_blocks().into_par_iter().flat_map(|it| {
            Block::from_block(
                it.text,
                it.range.start.line as usize,
                it.range.end.character as usize,
                it.file,
            )
        });

        let matched = run_query(
            link_query.to_query_string(),
            blocks.collect::<Vec<_>>().into_iter(),
        );

        matched
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

// TODO: Why is this not working???????? in the funcntiosn
trait Query {
    fn to_query_string(&self) -> String;
}

trait Queryable {
    fn match_string(&self) -> String;
}

// TODO: parallelize this
fn run_query<'a, 'b, B: Queryable + Send + 'a>(
    query: String,
    items: impl Iterator<Item = B> + 'a,
) -> impl IndexedParallelIterator<Item = B> + 'a {
    let items = items.map(|it| NucleoMatchable::from_queryable(it));

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let parser =
        pattern::Pattern::parse(&query, pattern::CaseMatching::Smart, Normalization::Smart);
    let matches = parser.match_list(items, &mut matcher);

    matches
        .into_par_iter()
        .map(|(item, _score)| item.to_queryable())
}

impl Query for BlockQuery<'_> {
    fn to_query_string(&self) -> String {
        self.grep_string.to_string()
    }
}

impl Query for EntityQuery<'_> {
    fn to_query_string(&self) -> String {
        match self {
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
}

impl Queryable for Entity<'_> {
    fn match_string(&self) -> String {
        let file_ref = self.info.path.file_name().unwrap().to_str().unwrap();

        match self.info.type_info {
            Heading(heading) => format!("{file_ref}#{heading}"),
            IndexedBlock(index) => format!("{file_ref}#^{index}"),
            _ => file_ref.to_string(),
        }
    }
}

impl Queryable for Block<'_> {
    fn match_string(&self) -> String {
        self.info.line_text.to_string()
    }
}

struct NucleoMatchable<T: Queryable + Send>(String, T);

impl<T: Queryable + Send> NucleoMatchable<T> {
    fn from_queryable(value: T) -> Self {
        NucleoMatchable(value.match_string(), value)
    }

    fn to_queryable(self) -> T {
        self.1
    }
}

impl<T: Queryable + Send> AsRef<str> for NucleoMatchable<T> {
    fn as_ref(&self) -> &str {
        &self.0
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
