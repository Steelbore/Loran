// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![forbid(unsafe_code)]

//! Loran core — orchestration and resolution chains.
//!
//! v1 surface:
//!
//! - [`BundledPagesIngestor`] — read-side source over the pages baked
//!   into the binary at compile time.
//! - [`resolve_show`] / [`ShowResult`] — curated-or-fail tool lookup
//!   for `loran show <tool>`.
//! - [`resolve_find`] / [`FindResult`] — reverse legacy-name lookup
//!   for `loran find <legacy>` (with optional `--safe-alias` filter).
//! - [`resolve_search`] / [`SearchResult`] / [`ScoredMatch`] — fuzzy
//!   match across the catalog for `loran search <query>`.
//!
//! The live `--help` capture engine and the tldr-pages tarball
//! resolver land in Sub-phase 1D and Phase 2 (Billet) respectively.

mod bundled;
mod bundled_data;
mod cache;
mod categories;
mod find;
mod help;
mod search;
mod show;
mod signing;

pub use bundled::BundledPagesIngestor;
pub use cache::{CACHE_FORMAT_VERSION, Cache, CacheError};
pub use categories::{Categories, CategoryEntry, bundled_categories};
pub use find::{FindResult, resolve_find};
pub use help::{
    HELP_TIMEOUT, HelpError, HelpFlag, HelpOpts, HelpResult, PagerSource, capture_help,
    resolve_pager,
};
pub use search::{ScoredMatch, SearchResult, resolve_search};
pub use show::{BodyBlock, IntroBlock, ShowResult, resolve_show};
pub use signing::{SignError, verify as verify_minisign};
