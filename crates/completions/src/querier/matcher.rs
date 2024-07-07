use nucleo_matcher::pattern;
use nucleo_matcher::pattern::Normalization;

use nucleo_matcher::Matcher;
use rayon::prelude::*;

pub fn run_query_on_par_iter<'a, T: Queryable + Send + 'a>(
    query: &'a impl Query,
    items: impl ParallelIterator<Item = T>,
) -> impl IndexedParallelIterator<Item = T> + 'a {
    run_query(query, items.collect::<Vec<_>>().into_iter())
}

pub(crate) fn run_query<'a, 'b, B: Queryable + Send + 'a>(
    query: &'b impl Query,
    items: impl Iterator<Item = B> + 'a,
) -> impl IndexedParallelIterator<Item = B> + 'a {
    let items = items.map(|it| NucleoMatchable::from_queryable(it));

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let parser = pattern::Pattern::parse(
        &query.to_query_string(),
        pattern::CaseMatching::Smart,
        Normalization::Smart,
    );
    let matches = parser.match_list(items, &mut matcher);

    matches
        .into_par_iter()
        .map(|(item, _score)| item.to_queryable())
}

// TODO: Why is this not working???????? in the funcntiosn
pub trait Query {
    fn to_query_string(&self) -> String;
}

pub(crate) trait Queryable {
    fn match_string(&self) -> String;
}

// TODO: parallelize this

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
