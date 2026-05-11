// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Errors surfaced by [`crate::Page::parse`].

use thiserror::Error;

/// Every way a Loran page can fail to parse.
///
/// Variants are deliberately fine-grained so the caller (Phase 1
/// `loran validate`, the index builder, `loran new --check`) can present
/// actionable diagnostics rather than a generic "parse failed".
#[derive(Debug, Error)]
pub enum PageError {
    /// No `+++` frontmatter fence pair was found.
    ///
    /// A Loran page **must** open with a `+++` line followed by a TOML
    /// block and a closing `+++` line. Files without that envelope are
    /// rejected outright — there is no implicit-frontmatter fallback.
    #[error("page is missing a `+++`-fenced TOML frontmatter block")]
    MissingFrontmatter,

    /// The frontmatter fence was present but the TOML between the
    /// fences did not deserialise.
    ///
    /// Carries the underlying `toml::de::Error` so the caller can show
    /// the user the original parser message (which already includes
    /// line and column context).
    #[error("frontmatter is not valid TOML: {0}")]
    InvalidToml(#[from] toml::de::Error),

    /// A required field was missing from the frontmatter.
    ///
    /// The required set is `name`, `category`, `summary` (Spec §6.1).
    #[error("frontmatter is missing required field `{0}`")]
    MissingField(&'static str),

    /// `summary` exceeded the 120-character cap.
    ///
    /// The Steelbore Standard requires summaries to fit cleanly into a
    /// single terminal line on an 80-column display with padding for
    /// formatting — 120 characters is the published ceiling (Spec §6.1).
    #[error("`summary` is {actual_chars} characters; the limit is 120")]
    SummaryTooLong {
        /// Length of the offending summary, in Unicode scalar values.
        actual_chars: usize,
    },

    /// An entry in `safe_alias_for` does not appear in `replaces`.
    ///
    /// `safe_alias_for` is a strict subset of `replaces` (Spec §2
    /// decision #14): a tool can only be "safe to alias to" if the entry
    /// also declares it as something it replaces. Violations indicate a
    /// page that promises more than its `replaces` set backs up.
    #[error("`safe_alias_for` entry `{0}` is not present in `replaces`")]
    InvalidSafeAliasFor(String),

    /// `category` is structurally malformed.
    ///
    /// Categories are slash-tolerant — `system/file-listing` is valid —
    /// but may not be empty, may not start or end with `/`, and may not
    /// contain `//`. The `reason` payload spells out which rule fired.
    #[error("`category` is malformed (`{value}`): {reason}")]
    InvalidCategory {
        /// The rejected category string, verbatim.
        value: String,
        /// Human-readable explanation of which well-formedness rule fired.
        reason: &'static str,
    },
}
