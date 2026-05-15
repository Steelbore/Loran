// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran show <tool>` resolution chain — curated-or-fail per Spec §4.1.
//!
//! There is no live `--help` fallback. If the tool is not in the index,
//! the caller (the `loran` CLI's `show` handler) must surface the
//! [`ShowResult::NoEntry`] hint and exit with `NOT_FOUND`. Live capture
//! lives behind the separate `loran help` verb (Spec §2 decision #7).

use loran_index::Index;
use loran_pages::Page;
use schemars::JsonSchema;
use serde::Serialize;

use crate::tldr::{NoTldr, TldrLookup};

/// Outcome of [`resolve_show`].
///
/// The `IndexHit` variant is much larger than `NoEntry` (it carries a
/// full `Page`); rather than box the larger variant we accept the size
/// difference because the result is always consumed immediately on the
/// stack at the call site and never stored in a long-lived collection.
#[derive(Debug, Clone, Serialize, JsonSchema)]
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

/// "Spacecraft Software intro" block surfaced as `data.intro` in the JSON envelope.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct IntroBlock {
    /// Always `"spacecraft software"` in v1.
    pub source: &'static str,
    /// Intro markdown — a paragraph derived from the page's `summary`.
    pub body_md: String,
}

/// "Body" block surfaced as `data.body` in the JSON envelope.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BodyBlock {
    /// `"custom"` when sourced from a curated Loran page, `"tldr"` when
    /// sourced from the tldr-pages mirror as a fallback,
    /// `"live_help"` for the [`crate::capture_help`] surface (used by
    /// the separate `loran help` verb, never by `resolve_show`).
    pub kind: &'static str,
    /// The full markdown body, verbatim from the resolved source.
    pub body_md: String,
    /// Whether a tldr-pages entry is plausibly available for this tool.
    /// `true` when the page's `tldr_page` field is set or defaults; the
    /// resolver consults [`TldrLookup`] to actually confirm and may
    /// flip this to reflect the real cache state.
    pub tldr_available: bool,
}

/// Resolve `tool` against the merged catalog index.
///
/// Equivalent to [`resolve_show_with_tldr`] with a [`NoTldr`] stub —
/// keeps the simple two-arg shape for callers (tests, the bundled-
/// pages-only path) that don't have a tldr cache.
#[must_use]
pub fn resolve_show(index: &Index, tool: &str) -> ShowResult {
    resolve_show_with_tldr(index, tool, &NoTldr)
}

/// Resolve `tool` against the merged catalog index, falling back to
/// the tldr-pages mirror via `tldr` when the curated body is empty.
///
/// Body resolution:
///
/// 1. **Curated body is non-empty** → `body.kind = "custom"`,
///    `body.body_md = page.body`. Default path; nothing changes from
///    the bundled-pages-only world.
/// 2. **Curated body is empty AND tldr hit** → `body.kind = "tldr"`,
///    `body.body_md` = the tldr text, `tldr_available = true`.
/// 3. **Curated body is empty AND tldr miss** → `body.kind = "custom"`
///    with an empty `body_md`; `tldr_available` reflects whether the
///    page's `tldr_page` field allows a future fallback.
///
/// Indexing: a tldr lookup is only attempted when the page's
/// `tldr_page` field is unset or set to a non-empty value (an explicit
/// empty string `tldr_page = ""` disables tldr per Spec §6.1). When
/// `Page::tldr_page` is `Some("rg")` the lookup uses `"rg"`; when
/// unset, the lookup uses `Page::name`.
#[must_use]
pub fn resolve_show_with_tldr(index: &Index, tool: &str, tldr: &dyn TldrLookup) -> ShowResult {
    let Some(page) = index.get(tool) else {
        return ShowResult::NoEntry {
            tool: tool.to_owned(),
            hint: format!("loran new {tool} --edit"),
        };
    };

    let intro = IntroBlock {
        source: "spacecraft software",
        body_md: format!("{}\n\nFrom Spacecraft Software curation.", page.summary),
    };

    let tldr_enabled = tldr_plausibly_available(page);
    let body = resolve_body(page, tldr, tldr_enabled);

    ShowResult::IndexHit {
        page: page.clone(),
        intro,
        body,
    }
}

/// Implement the body-resolution table from [`resolve_show_with_tldr`]'s
/// doc-comment. Returns the [`BodyBlock`] to surface in the result.
fn resolve_body(page: &Page, tldr: &dyn TldrLookup, tldr_enabled: bool) -> BodyBlock {
    // 1. Curated body present → use it.
    if !page.body.is_empty() {
        return BodyBlock {
            kind: "custom",
            body_md: page.body.clone(),
            tldr_available: tldr_enabled,
        };
    }
    // 2. Curated body empty + tldr enabled + tldr hit → fall back.
    if tldr_enabled {
        let lookup_key = page
            .tldr_page
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(page.name.as_str());
        if let Some(body_md) = tldr.lookup(lookup_key) {
            return BodyBlock {
                kind: "tldr",
                body_md,
                tldr_available: true,
            };
        }
    }
    // 3. No body resolved.
    BodyBlock {
        kind: "custom",
        body_md: page.body.clone(),
        tldr_available: tldr_enabled,
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
                assert_eq!(intro.source, "spacecraft software");
                assert!(intro.body_md.starts_with("the summary"));
                assert!(intro.body_md.contains("From Spacecraft Software curation"));
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

    // ─── tldr fallback tests ───────────────────────────────────────

    use super::resolve_show_with_tldr;
    use crate::tldr::TldrLookup;

    /// Canned `TldrLookup` mapping `tool → body_md`.
    struct CannedTldr(std::collections::HashMap<String, String>);

    impl CannedTldr {
        fn new(entries: &[(&str, &str)]) -> Self {
            Self(
                entries
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            )
        }
    }

    impl TldrLookup for CannedTldr {
        fn lookup(&self, tool: &str) -> Option<String> {
            self.0.get(tool).cloned()
        }
    }

    fn empty_body_page_src(name: &str) -> String {
        // Body intentionally empty after the closing `+++`.
        format!("+++\nname = \"{name}\"\ncategory = \"c\"\nsummary = \"s\"\n+++\n")
    }

    #[test]
    fn curated_body_present_uses_custom_kind_ignoring_tldr() {
        let idx = idx_with(&["eza"]); // page_src() leaves "body\n" as the body
        let tldr = CannedTldr::new(&[("eza", "tldr text")]);
        if let ShowResult::IndexHit { body, .. } = resolve_show_with_tldr(&idx, "eza", &tldr) {
            assert_eq!(body.kind, "custom", "curated takes precedence over tldr");
            assert!(body.body_md.contains("body"));
        } else {
            panic!("expected hit");
        }
    }

    #[test]
    fn empty_curated_body_with_tldr_hit_uses_tldr_kind() {
        let page = Page::parse(&empty_body_page_src("eza")).unwrap();
        let idx = Index::build(vec![page]).unwrap();
        let tldr = CannedTldr::new(&[("eza", "# eza (tldr)\n\nModern ls.\n")]);

        if let ShowResult::IndexHit { body, .. } = resolve_show_with_tldr(&idx, "eza", &tldr) {
            assert_eq!(body.kind, "tldr");
            assert!(body.body_md.contains("Modern ls"));
            assert!(body.tldr_available);
        } else {
            panic!("expected hit");
        }
    }

    #[test]
    fn empty_curated_body_with_tldr_miss_falls_through_to_empty_custom() {
        let page = Page::parse(&empty_body_page_src("widgetctl")).unwrap();
        let idx = Index::build(vec![page]).unwrap();
        let tldr = CannedTldr::new(&[]); // no entries

        if let ShowResult::IndexHit { body, .. } = resolve_show_with_tldr(&idx, "widgetctl", &tldr)
        {
            assert_eq!(body.kind, "custom");
            assert!(body.body_md.is_empty());
            assert!(body.tldr_available); // tldr_page default-on; the lookup just missed
        } else {
            panic!("expected hit");
        }
    }

    #[test]
    fn tldr_page_field_overrides_lookup_key() {
        // ripgrep page declares its tldr key as "rg" (the binary name).
        let src =
            "+++\nname = \"ripgrep\"\ncategory = \"c\"\nsummary = \"s\"\ntldr_page = \"rg\"\n+++\n";
        let idx = Index::build(vec![Page::parse(src).unwrap()]).unwrap();
        let tldr = CannedTldr::new(&[("rg", "# rg")]);

        if let ShowResult::IndexHit { body, .. } = resolve_show_with_tldr(&idx, "ripgrep", &tldr) {
            assert_eq!(body.kind, "tldr", "must use the tldr_page override key");
        } else {
            panic!("expected hit");
        }
    }

    #[test]
    fn tldr_page_explicitly_empty_disables_lookup() {
        let src = "+++\nname = \"x\"\ncategory = \"c\"\nsummary = \"s\"\ntldr_page = \"\"\n+++\n";
        let idx = Index::build(vec![Page::parse(src).unwrap()]).unwrap();
        let tldr = CannedTldr::new(&[("x", "should not be used")]);

        if let ShowResult::IndexHit { body, .. } = resolve_show_with_tldr(&idx, "x", &tldr) {
            assert_eq!(
                body.kind, "custom",
                "tldr_page=\"\" must suppress the fallback even with a tldr hit"
            );
            assert!(!body.tldr_available);
        } else {
            panic!("expected hit");
        }
    }

    #[test]
    fn no_tldr_stub_keeps_existing_resolve_show_behaviour_unchanged() {
        let page = Page::parse(&empty_body_page_src("eza")).unwrap();
        let idx = Index::build(vec![page]).unwrap();
        // The plain resolve_show uses NoTldr internally.
        if let ShowResult::IndexHit { body, .. } = resolve_show(&idx, "eza") {
            assert_eq!(body.kind, "custom");
            assert!(body.body_md.is_empty());
        } else {
            panic!("expected hit");
        }
    }
}
