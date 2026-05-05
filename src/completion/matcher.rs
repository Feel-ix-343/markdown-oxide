use std::ops::Deref;

use nucleo_matcher::{
    pattern::{self, Normalization},
    Matcher,
};
use tower_lsp::lsp_types::CompletionItem;

use crate::config::Case;

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

pub struct OrderedCompletion<'a, C, T>
where
    C: Completer<'a>,
    T: Completable<'a, C>,
{
    completable: T,
    rank: String,
    __phantom: std::marker::PhantomData<&'a T>,
    __phantom2: std::marker::PhantomData<C>,
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

impl<'a, C: Completer<'a>, T: Completable<'a, C>> Completable<'a, C>
    for OrderedCompletion<'a, C, T>
{
    fn completions(&self, completer: &C) -> Option<CompletionItem> {
        let completion = self.completable.completions(completer);

        completion.map(|completion| CompletionItem {
            sort_text: Some(self.rank.to_string()),
            ..completion
        })
    }
}

pub fn fuzzy_match_completions<'a, 'b, C: Completer<'a>, T: Matchable + Completable<'a, C>>(
    filter_text: &'b str,
    items: impl IntoIterator<Item = T>,
    case: &Case,
) -> Vec<OrderedCompletion<'a, C, T>> {
    let normal_fuzzy_match = fuzzy_match(filter_text, items, case);

    normal_fuzzy_match
        .into_iter()
        .map(|(item, score)| OrderedCompletion::new(item, score_to_sort_text(score)))
        .collect::<Vec<_>>()
}

fn score_to_sort_text(score: u32) -> String {
    score.to_string()
}

pub fn fuzzy_match<T: Matchable>(
    filter_text: &str,
    items: impl IntoIterator<Item = T>,
    case: &Case,
) -> Vec<(T, u32)> {
    let items = items.into_iter().map(NucleoMatchable);

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let matches = pattern::Pattern::parse(
        filter_text,
        match case {
            Case::Smart => pattern::CaseMatching::Smart,
            Case::Ignore => pattern::CaseMatching::Ignore,
            Case::Respect => pattern::CaseMatching::Respect,
        },
        Normalization::Smart,
    )
    .match_list(items, &mut matcher);

    matches
        .into_iter()
        .map(|(item, score)| (item.0, score))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::score_to_sort_text;

    #[test]
    fn higher_score_sorts_lexicographically_first() {
        // nucleo returns higher score = better match. LSP clients sort `sortText`
        // ascending, so a better match must produce a *smaller* string.
        let better = score_to_sort_text(1000);
        let worse = score_to_sort_text(10);
        assert!(
            better < worse,
            "better match (1000) should sort before worse (10): {better:?} vs {worse:?}"
        );
    }

    #[test]
    fn digit_count_does_not_invert_order() {
        // Without zero-padding, "100" is a prefix of "1000" so "100" < "1000"
        // lexicographically — but 1000 is the better score and must sort first.
        let s_1000 = score_to_sort_text(1000);
        let s_100 = score_to_sort_text(100);
        assert!(
            s_1000 < s_100,
            "score 1000 should sort before score 100: {s_1000:?} vs {s_100:?}"
        );
    }

    #[test]
    fn lexicographic_sort_matches_descending_score_order() {
        let mut entries: Vec<(u32, String)> = [9, 99, 100, 1000, 50, 7]
            .iter()
            .map(|&s| (s, score_to_sort_text(s)))
            .collect();
        entries.sort_by(|a, b| a.1.cmp(&b.1));
        let scores: Vec<u32> = entries.iter().map(|(s, _)| *s).collect();
        assert_eq!(scores, vec![1000, 100, 99, 50, 9, 7]);
    }
}
