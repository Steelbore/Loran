// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran search <query>` fuzzy match across the catalog.
//!
//! Each page's haystack is a `{name} {summary} {replaces…} {tags…}`
//! concatenation; `nucleo-matcher` scores the query against the
//! haystack and we sort descending by score with alphabetical
//! tiebreaking. No `--limit` flag in v1 — the bundled catalog is
//! small enough that returning everything matched is the right
//! default. Phase 2 may add `--limit` once the catalog grows.

use loran_index::Index;
use loran_pages::Page;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32String};
use schemars::JsonSchema;
use serde::Serialize;

/// A single fuzzy-search hit, paired with its match score.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScoredMatch {
    /// The matched page in full.
    pub page: Page,
    /// `nucleo-matcher` score. Higher is better; meaningful only as a
    /// relative ordering. Surfaced in JSON output for agents that
    /// want to threshold.
    pub score: u32,
}

/// Outcome of [`resolve_search`].
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SearchResult {
    /// The free-text query the user typed, echoed back.
    pub query: String,
    /// Matches, sorted by score descending, with alphabetical
    /// tiebreaking on the canonical name.
    pub matches: Vec<ScoredMatch>,
}

/// Fuzzy-match `query` against the catalog.
///
/// The empty query short-circuits to an empty result — searching for
/// "nothing" is meaningless and `nucleo-matcher` would happily return
/// every entry with score zero in that case, which is worse UX than
/// returning nothing.
#[must_use]
pub fn resolve_search(index: &Index, query: &str) -> SearchResult {
    if query.trim().is_empty() {
        return SearchResult {
            query: query.to_owned(),
            matches: Vec::new(),
        };
    }

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

    let mut scored: Vec<ScoredMatch> = index
        .all()
        .filter_map(|page| {
            let haystack = Utf32String::from(build_haystack(page));
            pattern
                .score(haystack.slice(..), &mut matcher)
                .map(|score| ScoredMatch {
                    page: page.clone(),
                    score,
                })
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.page.name.cmp(&b.page.name))
    });

    SearchResult {
        query: query.to_owned(),
        matches: scored,
    }
}

/// Build the searchable string for a page: `name` + `summary` +
/// `replaces` + `tags`, space-joined.
fn build_haystack(page: &Page) -> String {
    let mut s = String::with_capacity(
        page.name.len() + page.summary.len() + page.replaces.iter().map(String::len).sum::<usize>(),
    );
    s.push_str(&page.name);
    s.push(' ');
    s.push_str(&page.summary);
    for r in &page.replaces {
        s.push(' ');
        s.push_str(r);
    }
    for t in &page.tags {
        s.push(' ');
        s.push_str(t);
    }
    s
}

#[cfg(test)]
mod tests {
    use loran_index::Index;
    use loran_pages::Page;

    use super::resolve_search;

    fn make_page(name: &str, summary: &str, replaces: &[&str], tags: &[&str]) -> Page {
        let replaces_lit = replaces
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let tags_lit = tags
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let src = format!(
            "+++\nname = \"{name}\"\ncategory = \"c\"\nsummary = \"{summary}\"\n\
             replaces = [{replaces_lit}]\ntags = [{tags_lit}]\n+++\n"
        );
        Page::parse(&src).expect("test page")
    }

    fn idx() -> Index {
        Index::build(vec![
            make_page("eza", "modern ls replacement", &["ls"], &["filesystem"]),
            make_page(
                "bat",
                "cat with wings and syntax",
                &["cat"],
                &["filesystem"],
            ),
            make_page("rg", "ripgrep recursive regex", &["grep"], &["search"]),
            make_page("fd", "find replacement", &["find"], &["filesystem"]),
        ])
        .unwrap()
    }

    #[test]
    fn name_match_returns_a_top_hit() {
        let result = resolve_search(&idx(), "eza");
        assert_eq!(result.query, "eza");
        assert!(!result.matches.is_empty(), "eza must match itself");
        assert_eq!(result.matches[0].page.name, "eza");
    }

    #[test]
    fn replaces_match_finds_the_modern_entry() {
        let result = resolve_search(&idx(), "grep");
        let names: Vec<&str> = result
            .matches
            .iter()
            .map(|m| m.page.name.as_str())
            .collect();
        assert!(
            names.contains(&"rg"),
            "grep search should hit rg via `replaces`: {names:?}"
        );
    }

    #[test]
    fn tag_match_finds_all_tag_holders() {
        let result = resolve_search(&idx(), "filesystem");
        let names: Vec<&str> = result
            .matches
            .iter()
            .map(|m| m.page.name.as_str())
            .collect();
        for expected in ["eza", "bat", "fd"] {
            assert!(
                names.contains(&expected),
                "filesystem tag should hit {expected}: {names:?}"
            );
        }
    }

    #[test]
    fn matches_are_sorted_by_score_descending() {
        let result = resolve_search(&idx(), "filesystem");
        for pair in result.matches.windows(2) {
            assert!(
                pair[0].score >= pair[1].score,
                "scores must descend: got {} then {}",
                pair[0].score,
                pair[1].score
            );
        }
    }

    #[test]
    fn empty_query_returns_empty_result_without_panic() {
        let result = resolve_search(&idx(), "");
        assert!(result.matches.is_empty());
    }

    #[test]
    fn whitespace_only_query_returns_empty_result() {
        let result = resolve_search(&idx(), "    ");
        assert!(result.matches.is_empty());
    }

    #[test]
    fn unicode_query_does_not_panic() {
        let result = resolve_search(&idx(), "🚀 search");
        let _ = result.matches.len();
    }

    #[test]
    fn very_long_query_does_not_panic() {
        let long: String = "x".repeat(10_000);
        let result = resolve_search(&idx(), &long);
        let _ = result.matches.len();
    }
}
