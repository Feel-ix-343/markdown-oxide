use std::ops::Deref;

use nucleo_matcher::{pattern::{self, Normalization}, Matcher};
use tower_lsp::lsp_types::CompletionItem;


use super::{Completable, Completer};

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


pub struct OrderedCompletion<'a, C, T> where C: Completer<'a>, T: Completable<'a, C> {
    completable: T,
    rank: String,
    __phantom: std::marker::PhantomData<&'a T>,
    __phantom2: std::marker::PhantomData<C>
}

impl<'a, C: Completer<'a>, T: Completable<'a, C>> OrderedCompletion<'a, C, T> {
    pub fn new(completable: T, rank: String) -> Self {
        Self {
            completable,
            rank,
            __phantom: std::marker::PhantomData,
            __phantom2: std::marker::PhantomData,
        }
    }
}

impl<'a, C: Completer<'a>, T: Completable<'a, C>> Completable<'a, C> for OrderedCompletion<'a, C, T> {
    fn completions(&self, completer: &C) -> impl Iterator<Item = tower_lsp::lsp_types::CompletionItem> {
        let completions = self.completable.completions(completer);

        completions.map(|completion| CompletionItem {
            sort_text: Some(self.rank.to_string()),
            ..completion
        })
    }
}

pub fn fuzzy_match_completions<'a, 'b, C: Completer<'a>, T: Matchable + Completable<'a, C>>(
    filter_text: &'b str,
    items: impl IntoIterator<Item = T>
) -> Vec<OrderedCompletion<'a, C, T>> {

    let normal_fuzzy_match = fuzzy_match(filter_text, items);

    normal_fuzzy_match
        .into_iter()
        .map(|(item, score)| OrderedCompletion::new(item, score.to_string()))
        .collect::<Vec<_>>()

}

pub fn fuzzy_match<'a, T: Matchable>(
    filter_text: &str,
    items: impl IntoIterator<Item = T>,
) -> Vec<(T, u32)> {

    let items = items.into_iter().map(NucleoMatchable);

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let matches = pattern::Pattern::parse(
        filter_text,
        pattern::CaseMatching::Ignore,
        Normalization::Smart,
    )
    .match_list(items, &mut matcher);

    matches
        .into_iter()
        .map(|(item, score)| (item.0, score))
        .collect()
}
