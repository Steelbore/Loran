// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran find <legacy>` reverse lookup — "which Loran-blessed tool
//! supersedes the legacy name I know?"
//!
//! Two modes per Spec §7:
//!
//! - Broad (default): any entry whose `replaces` list mentions
//!   `legacy`. Returns the full set so the user can see the
//!   recommendation landscape.
//! - Strict (`safe_alias_only = true`, surfaced as `--safe-alias` on
//!   the CLI): only entries whose `safe_alias_for` mentions `legacy`.
//!   These are the entries the user can `alias legacy=modern` without
//!   breaking common-case scripts.

use loran_index::Index;
use loran_pages::Page;
use serde::Serialize;

/// Outcome of [`resolve_find`].
#[derive(Debug, Clone, Serialize)]
pub struct FindResult {
    /// The legacy tool name the user asked about, echoed back.
    pub query: String,
    /// `true` when the result set was filtered down to alias-safe
    /// matches only (the `--safe-alias` invocation).
    pub safe_alias_only: bool,
    /// Matched pages, alphabetically sorted by canonical name.
    pub matches: Vec<Page>,
}

/// Resolve `legacy` against the merged catalog index.
#[must_use]
pub fn resolve_find(index: &Index, legacy: &str, safe_alias_only: bool) -> FindResult {
    let mut matches: Vec<Page> = if safe_alias_only {
        index.by_safe_alias(legacy).cloned().collect()
    } else {
        index.by_replaces(legacy).cloned().collect()
    };
    matches.sort_by(|a, b| a.name.cmp(&b.name));

    FindResult {
        query: legacy.to_owned(),
        safe_alias_only,
        matches,
    }
}

#[cfg(test)]
mod tests {
    use loran_index::Index;
    use loran_pages::Page;

    use super::resolve_find;

    fn make_page(name: &str, replaces: &[&str], safe: &[&str]) -> Page {
        let replaces_lit = replaces
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let safe_lit = safe
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let src = format!(
            "+++\nname = \"{name}\"\ncategory = \"c\"\nsummary = \"s\"\n\
             replaces = [{replaces_lit}]\nsafe_alias_for = [{safe_lit}]\n+++\n"
        );
        Page::parse(&src).expect("test page")
    }

    #[test]
    fn broad_mode_finds_every_replacer() {
        let idx = Index::build(vec![
            make_page("rg", &["grep"], &[]),
            make_page("ack", &["grep"], &[]),
            make_page("bat", &["cat"], &["cat"]),
        ])
        .unwrap();

        let result = resolve_find(&idx, "grep", false);
        assert!(!result.safe_alias_only);
        assert_eq!(result.query, "grep");
        let names: Vec<&str> = result.matches.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["ack", "rg"], "alphabetical sort");
    }

    #[test]
    fn strict_mode_only_returns_alias_safe_entries() {
        let idx = Index::build(vec![
            make_page("rg", &["grep"], &[]),
            make_page("ack", &["grep"], &[]),
            make_page("bat", &["cat"], &["cat"]),
            make_page("jaq", &["jq"], &["jq"]),
        ])
        .unwrap();

        // No grep replacer is alias-safe.
        let grep_strict = resolve_find(&idx, "grep", true);
        assert!(grep_strict.safe_alias_only);
        assert!(grep_strict.matches.is_empty());

        // cat → bat is alias-safe.
        let cat_strict = resolve_find(&idx, "cat", true);
        let names: Vec<&str> = cat_strict.matches.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["bat"]);
    }

    #[test]
    fn empty_query_returns_empty_result_without_panic() {
        let idx = Index::build(vec![make_page("rg", &["grep"], &[])]).unwrap();
        let result = resolve_find(&idx, "", false);
        assert_eq!(result.query, "");
        assert!(result.matches.is_empty());
    }

    #[test]
    fn unknown_legacy_returns_empty_result() {
        let idx = Index::build(vec![make_page("rg", &["grep"], &[])]).unwrap();
        let result = resolve_find(&idx, "no-such-legacy-tool", false);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn unicode_query_does_not_panic() {
        let idx = Index::build(vec![make_page("rg", &["grep"], &[])]).unwrap();
        let result = resolve_find(&idx, "ls — but emoji 🚀", false);
        assert!(result.matches.is_empty());
    }
}
