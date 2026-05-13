// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Metadata-only projections of `loran-pages::Page` for CLI output.
//!
//! `Page` now carries its raw markdown body through serde alongside
//! every other field — that's required for the postcard cache
//! (`loran-core::cache`) and any future consumer that wants to
//! round-trip a `Page` losslessly. The CLI's JSON output shapes
//! (Spec §8 list, find, search) deliberately omit the body so the
//! payloads stay compact for agents iterating the catalog.
//!
//! This module exposes [`PageSummary`], a borrowed view that
//! `Serialize`s every metadata field except `body`. Sub-command
//! handlers project to it when emitting JSON.

use loran_pages::Page;
use serde::Serialize;

/// Metadata-only borrowed view of a `Page`. Excludes the raw markdown
/// body so list-shaped JSON output stays compact.
#[derive(Serialize)]
pub(crate) struct PageSummary<'a> {
    pub name: &'a str,
    pub category: &'a str,
    pub summary: &'a str,
    pub replaces: &'a [String],
    pub safe_alias_for: &'a [String],
    pub pairs_with: &'a [String],
    pub official: Option<&'a str>,
    pub tldr_page: Option<&'a str>,
    pub tags: &'a [String],
    pub written_in: Option<&'a str>,
    pub language: Option<&'a str>,
    pub since: Option<&'a str>,
    pub aliases: &'a [String],
}

impl<'a> From<&'a Page> for PageSummary<'a> {
    fn from(page: &'a Page) -> Self {
        Self {
            name: &page.name,
            category: &page.category,
            summary: &page.summary,
            replaces: page.replaces.as_slice(),
            safe_alias_for: page.safe_alias_for.as_slice(),
            pairs_with: page.pairs_with.as_slice(),
            official: page.official.as_deref(),
            tldr_page: page.tldr_page.as_deref(),
            tags: page.tags.as_slice(),
            written_in: page.written_in.as_deref(),
            language: page.language.as_deref(),
            since: page.since.as_deref(),
            aliases: page.aliases.as_slice(),
        }
    }
}
