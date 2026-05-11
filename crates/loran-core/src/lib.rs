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
mod find;
mod search;
mod show;

pub use bundled::BundledPagesIngestor;
pub use find::{FindResult, resolve_find};
pub use search::{ScoredMatch, SearchResult, resolve_search};
pub use show::{BodyBlock, IntroBlock, ShowResult, resolve_show};
