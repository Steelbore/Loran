// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![forbid(unsafe_code)]

//! Loran renderer — Markdown body → terminal output.
//!
//! v1 ships a single mode: ANSI-free plain text suitable for any
//! POSIX terminal **and** for piping into `grep` / `awk` / `cut` / `sed`
//! (PRD quality goal Q-03, NFR-050). TUI rendering with the Spacecraft Software
//! palette is deferred to Phase 2 (Billet).
//!
//! Output rules (Spec §10, kept minimal so the same renderer can drive
//! both `loran show` and the rare cases where we render a tldr body):
//!
//! - Headings become a blank line, the heading text in `UPPER CASE`
//!   (Standard "primary text" convention for non-coloured surfaces),
//!   another blank line.
//! - Paragraphs become a text line followed by a blank line.
//! - Lists are emitted with `- ` bullets, with two spaces of
//!   indentation per nesting level. Numbered lists are still bullet-
//!   formatted; numerical ordering is not preserved in v1 because the
//!   Spacecraft Software catalog content does not rely on it.
//! - Code blocks indent every contained line by four spaces. Fence
//!   info-strings are dropped.
//! - Inline code becomes `` `text` ``.
//! - Links become `text (url)` — visible URLs for terminal users who
//!   cannot click.
//! - Raw HTML is passed through verbatim.

mod text;

pub use text::render_text;
