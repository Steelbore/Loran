// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![forbid(unsafe_code)]

//! Loran page parser.
//!
//! A Loran page is a single Markdown file with a TOML frontmatter block
//! fenced by `+++` (Hugo / Zola style). Frontmatter declares structured
//! metadata about a tool — its canonical name, the legacy tools it
//! replaces, a short summary, a category, optional companion tools, and
//! so on. Everything after the closing `+++` is treated as the page body
//! and stored verbatim for later rendering.
//!
//! Parsing is strict by design (Spec §6.1: "Index build fails loud on
//! schema violations — no silent skipping"):
//!
//! - Unknown frontmatter fields are rejected (typos become errors).
//! - Required fields (`name`, `category`, `summary`) must be present.
//! - `summary` is capped at 120 characters.
//! - `safe_alias_for` must be a subset of `replaces` — a page that
//!   declares an alias-safe legacy tool the entry doesn't actually
//!   replace is malformed by definition.
//! - `category` is slash-tolerant (`system/file-listing` is valid) but
//!   may not start or end with `/` and may not contain `//`.
//!
//! [`Page::parse`] is the single public entry point; it returns a
//! validated [`Page`] or a typed [`PageError`] describing exactly what
//! went wrong.

mod error;
mod overlay;
mod page;
mod parse;

pub use error::PageError;
pub use overlay::OverlayPage;
pub use page::Page;
