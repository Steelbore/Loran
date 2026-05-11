// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran show <tool>` resolution chain — curated-or-fail per Spec §4.1.
//!
//! There is no live `--help` fallback. If the tool is not in the index,
//! the caller (the `loran-cli` `show` handler) must surface the
//! [`ShowResult::NoEntry`] hint and exit with `NOT_FOUND`. Live capture
//! lives behind the separate `loran help` verb (Spec §2 decision #7).

use loran_index::Index;
use loran_pages::Page;
use serde::Serialize;

/// Outcome of [`resolve_show`].
///
/// The `IndexHit` variant is much larger than `NoEntry` (it carries a
/// full `Page`); rather than box the larger variant we accept the size
/// difference because the result is always consumed immediately on the
/// stack at the call site and never stored in a long-lived collection.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // consumed immediately at the call site
pub enum ShowResult {
    /// The tool was found in the index.
    IndexHit {
        /// The matched page in full.
        page: Page,
        /// Synthesised intro paragraph for v1 — derived from
        /// [`Page::summary`]. Future spec revisions may parse this out
        /// of a dedicated `[intro]` frontmatter section.
        intro: IntroBlock,
        /// The body block, ready for `loran-render`.
        body: BodyBlock,
    },
    /// No catalog entry for this tool.
    NoEntry {
        /// The tool name the caller asked about (echoed back so the
        /// error path can interpolate it into the hint).
        tool: String,
        /// Canonical hint string per Spec §4.1.1 — `loran new <tool>
        /// --edit`.
        hint: String,
    },
}

/// "Steelbore intro" block surfaced as `data.intro` in the JSON envelope.
#[derive(Debug, Clone, Serialize)]
pub struct IntroBlock {
    /// Always `"steelbore"` in v1.
    pub source: &'static str,
    /// Intro markdown — a paragraph derived from the page's `summary`.
    pub body_md: String,
}

/// "Body" block surfaced as `data.body` in the JSON envelope.
#[derive(Debug, Clone, Serialize)]
pub struct BodyBlock {
    /// `"custom"` when sourced from a curated Loran page. Future verbs
    /// add `"tldr"` and `"live_help"`; in v1 (Phase 1C scope) only
    /// `"custom"` is reachable from `resolve_show`.
    pub kind: &'static str,
    /// The full markdown body, verbatim from the page (`Page::body`).
    pub body_md: String,
    /// Whether a tldr-pages entry is plausibly available for this tool
    /// (the page's `tldr_page` field is set or defaults). The tldr
    /// resolver lands in Phase 2.
    pub tldr_available: bool,
}

/// Resolve `tool` against the merged catalog index.
#[must_use]
pub fn resolve_show(index: &Index, tool: &str) -> ShowResult {
    match index.get(tool) {
        Some(page) => {
            let intro = IntroBlock {
                source: "steelbore",
                body_md: format!("{}\n\nFrom Steelbore curation.", page.summary),
            };
            let body = BodyBlock {
                kind: "custom",
                body_md: page.body.clone(),
                tldr_available: tldr_plausibly_available(page),
            };
            ShowResult::IndexHit {
                page: page.clone(),
                intro,
                body,
            }
        }
        None => ShowResult::NoEntry {
            tool: tool.to_owned(),
            hint: format!("loran new {tool} --edit"),
        },
    }
}

/// A tldr lookup is plausibly available iff the page declares
/// `tldr_page` (any non-empty value) or omits it (which defaults to the
/// page's canonical name). An explicit empty string disables tldr.
fn tldr_plausibly_available(page: &Page) -> bool {
    match &page.tldr_page {
        None => true,
        Some(s) => !s.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use loran_index::Index;
    use loran_pages::Page;

    use super::{ShowResult, resolve_show};

    fn page_src(name: &str) -> String {
        format!("+++\nname = \"{name}\"\ncategory = \"c\"\nsummary = \"the summary\"\n+++\nbody\n")
    }

    fn idx_with(names: &[&str]) -> Index {
        let pages: Vec<Page> = names
            .iter()
            .map(|n| Page::parse(&page_src(n)).unwrap())
            .collect();
        Index::build(pages).unwrap()
    }

    #[test]
    fn hit_returns_index_hit_with_intro_derived_from_summary() {
        let idx = idx_with(&["eza"]);
        let result = resolve_show(&idx, "eza");
        match result {
            ShowResult::IndexHit { page, intro, body } => {
                assert_eq!(page.name, "eza");
                assert_eq!(intro.source, "steelbore");
                assert!(intro.body_md.starts_with("the summary"));
                assert!(intro.body_md.contains("From Steelbore curation"));
                assert_eq!(body.kind, "custom");
                assert!(body.body_md.contains("body"));
            }
            ShowResult::NoEntry { tool, hint } => {
                panic!("expected IndexHit, got NoEntry {{ tool={tool}, hint={hint} }}")
            }
        }
    }

    #[test]
    fn miss_returns_no_entry_with_canonical_hint() {
        let idx = idx_with(&["eza"]);
        let result = resolve_show(&idx, "widgetctl");
        match result {
            ShowResult::NoEntry { tool, hint } => {
                assert_eq!(tool, "widgetctl");
                assert_eq!(hint, "loran new widgetctl --edit");
            }
            ShowResult::IndexHit { page, .. } => {
                panic!("expected NoEntry, got IndexHit on page {}", page.name)
            }
        }
    }

    #[test]
    fn empty_query_does_not_panic_and_returns_no_entry() {
        let idx = idx_with(&["eza"]);
        let result = resolve_show(&idx, "");
        assert!(matches!(result, ShowResult::NoEntry { .. }));
    }

    #[test]
    fn unicode_query_does_not_panic() {
        let idx = idx_with(&["eza"]);
        let result = resolve_show(&idx, "🚀");
        assert!(matches!(result, ShowResult::NoEntry { .. }));
    }

    #[test]
    fn very_long_query_does_not_panic() {
        let idx = idx_with(&["eza"]);
        let long: String = "x".repeat(10_000);
        let result = resolve_show(&idx, &long);
        assert!(matches!(result, ShowResult::NoEntry { .. }));
    }

    #[test]
    fn tldr_available_defaults_to_true_when_field_absent() {
        let idx = idx_with(&["eza"]);
        let result = resolve_show(&idx, "eza");
        if let ShowResult::IndexHit { body, .. } = result {
            assert!(body.tldr_available);
        } else {
            panic!("expected hit");
        }
    }

    #[test]
    fn tldr_available_false_when_explicitly_empty() {
        let src = "+++\nname = \"x\"\ncategory = \"c\"\nsummary = \"s\"\ntldr_page = \"\"\n+++\n";
        let idx = Index::build(vec![Page::parse(src).unwrap()]).unwrap();
        if let ShowResult::IndexHit { body, .. } = resolve_show(&idx, "x") {
            assert!(!body.tldr_available);
        } else {
            panic!("expected hit");
        }
    }
}
