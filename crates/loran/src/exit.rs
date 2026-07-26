// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

// Exit-code constructors + hint paths are exercised by tests today and
// will be called from every error path starting in Sub-phase 1C.
#![allow(dead_code)]

//! Canonical exit-code map + tips-thinking error hints.
//!
//! Per `loran-spec-v0_2.md` §9, Loran reserves the CLI-Standard-canonical
//! codes 0–5 (`Success` / `GeneralError` / `UsageError` / `NotFound` /
//! `PermissionDenied` / `Conflict`) and adds six Loran-specific codes
//! 6–11. Every variant carries:
//!
//! - a stable string `name` (`"NOT_FOUND"`, `"INDEX_NOT_BUILT"`, …),
//!   surfaced as `error.code` in the CLI Standard's envelope and used by agents
//!   for branching;
//! - a `numeric` exit code, returned to the OS;
//! - a runnable `hint`, produced via [`ExitCode::hint`] over an
//!   [`ErrorContext`], following the "tips-thinking" discipline from
//!   `spacecraft-agentic-cli` §5 — every error response must show the
//!   user a command they can paste to recover.

use std::fmt;

/// Loran exit-code enum, one variant per documented code in Spec §9.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ExitCode {
    Success,
    GeneralError,
    UsageError,
    NotFound,
    PermissionDenied,
    Conflict,
    IndexNotBuilt,
    TarballFetchFailed,
    PageParseError,
    LiveHelpTimeout,
    OverlayWriteDenied,
    TarballVerifyFailed,
}

impl ExitCode {
    /// Numeric value passed to `process::exit` / surfaced as
    /// `error.exit_code` in the JSON envelope.
    pub(crate) fn numeric(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::GeneralError => 1,
            Self::UsageError => 2,
            Self::NotFound => 3,
            Self::PermissionDenied => 4,
            Self::Conflict => 5,
            Self::IndexNotBuilt => 6,
            Self::TarballFetchFailed => 7,
            Self::PageParseError => 8,
            Self::LiveHelpTimeout => 9,
            Self::OverlayWriteDenied => 10,
            Self::TarballVerifyFailed => 11,
        }
    }

    /// Stable string identifier surfaced as `error.code` in the CLI Standard's
    /// envelope. `SCREAMING_SNAKE_CASE` per Spec §9.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::GeneralError => "GENERAL_ERROR",
            Self::UsageError => "USAGE_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::Conflict => "CONFLICT",
            Self::IndexNotBuilt => "INDEX_NOT_BUILT",
            Self::TarballFetchFailed => "TARBALL_FETCH_FAILED",
            Self::PageParseError => "PAGE_PARSE_ERROR",
            Self::LiveHelpTimeout => "LIVE_HELP_TIMEOUT",
            Self::OverlayWriteDenied => "OVERLAY_WRITE_DENIED",
            Self::TarballVerifyFailed => "TARBALL_VERIFY_FAILED",
        }
    }

    /// Runnable recovery hint per Spec §12.3.
    ///
    /// Interpolates fields from `context` where the hint shape supports
    /// it (e.g. `NOT_FOUND` uses `context.query`). The returned `String`
    /// is intended to be pasted verbatim into a shell; do not wrap it
    /// in backticks or quotes when surfacing it in human output.
    pub(crate) fn hint(self, context: &ErrorContext) -> String {
        match self {
            Self::Success => String::new(),
            Self::GeneralError => "loran --help        # see the full command surface".to_owned(),
            Self::UsageError => "loran --help        # check argument shape".to_owned(),
            Self::NotFound => match (&context.query, &context.tool) {
                (Some(q), _) => format!("loran search {q} --json"),
                (None, Some(t)) => format!("loran search {t} --json"),
                _ => "loran list --json   # browse the full catalog".to_owned(),
            },
            Self::PermissionDenied => match &context.tool {
                Some(t) => format!(
                    "ls -ld $XDG_DATA_HOME/loran/overlays/user  # inspect, then re-run on {t}"
                ),
                None => "ls -ld $XDG_DATA_HOME/loran  # inspect permissions".to_owned(),
            },
            Self::Conflict => "loran update --dry-run  # preview the conflicting change".to_owned(),
            Self::IndexNotBuilt => "loran update".to_owned(),
            Self::TarballFetchFailed => {
                "loran update --force-refresh  # retry after checking network".to_owned()
            }
            Self::PageParseError => "loran validate --json".to_owned(),
            Self::LiveHelpTimeout => match &context.tool {
                Some(t) => format!("loran new {t} --edit"),
                None => "loran new <tool> --edit".to_owned(),
            },
            Self::OverlayWriteDenied => match &context.tool {
                Some(t) => {
                    format!("mkdir -p \"$XDG_DATA_HOME/loran/overlays/user\" && loran new {t}")
                }
                None => {
                    "mkdir -p \"$XDG_DATA_HOME/loran/overlays/user\" && loran new <tool>".to_owned()
                }
            },
            Self::TarballVerifyFailed => {
                "loran update --force-refresh  # only after confirming the publisher key has \
                 not rotated; otherwise upgrade Loran"
                    .to_owned()
            }
        }
    }

    /// True if this code represents a successful outcome.
    pub(crate) fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// Numeric exit code as a `u8`, suitable for [`std::process::ExitCode::from`].
    ///
    /// Every variant fits in `u8` by construction (0..=11), so the
    /// conversion is infallible — but it is expressed via `try_from`
    /// so future variant additions cannot silently truncate.
    pub(crate) fn to_process_code(self) -> u8 {
        u8::try_from(self.numeric()).unwrap_or(1)
    }

    /// Every variant, in numeric order. Used by tests and by `loran
    /// schema --json` (Phase 3) to enumerate the surface.
    pub(crate) fn all() -> [ExitCode; 12] {
        [
            Self::Success,
            Self::GeneralError,
            Self::UsageError,
            Self::NotFound,
            Self::PermissionDenied,
            Self::Conflict,
            Self::IndexNotBuilt,
            Self::TarballFetchFailed,
            Self::PageParseError,
            Self::LiveHelpTimeout,
            Self::OverlayWriteDenied,
            Self::TarballVerifyFailed,
        ]
    }
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name(), self.numeric())
    }
}

/// Context bag passed to [`ExitCode::hint`] so hints can interpolate
/// user-supplied values (the legacy tool name they searched for, the
/// canonical tool they tried to look up, etc.).
///
/// Fields are all optional; populate whichever the calling site has on
/// hand. Unmatched variants fall back to generic placeholders.
#[derive(Debug, Default, Clone)]
pub(crate) struct ErrorContext {
    /// The free-text query (e.g. for `loran search`, `loran find`).
    pub query: Option<String>,
    /// The canonical or legacy tool name in scope at error time.
    pub tool: Option<String>,
}

impl ErrorContext {
    /// Empty context — every hint falls back to its generic form.
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    /// Convenience for hints that interpolate a tool name.
    pub(crate) fn with_tool(tool: impl Into<String>) -> Self {
        Self {
            tool: Some(tool.into()),
            ..Self::default()
        }
    }

    /// Convenience for hints that interpolate a free-text query.
    pub(crate) fn with_query(query: impl Into<String>) -> Self {
        Self {
            query: Some(query.into()),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{ErrorContext, ExitCode};

    #[test]
    fn numeric_codes_cover_zero_through_eleven_without_gaps() {
        let numbers: HashSet<i32> = ExitCode::all().iter().map(|c| c.numeric()).collect();
        assert_eq!(numbers.len(), 12, "no duplicate numbers");
        for expected in 0..=11 {
            assert!(numbers.contains(&expected), "missing code {expected}");
        }
    }

    #[test]
    fn names_are_distinct_and_screaming_snake_case() {
        let names: HashSet<&'static str> = ExitCode::all().iter().map(|c| c.name()).collect();
        assert_eq!(names.len(), 12, "no duplicate names");
        for n in &names {
            assert!(
                n.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "{n} must be SCREAMING_SNAKE_CASE"
            );
        }
    }

    #[test]
    fn every_non_success_variant_produces_a_non_empty_generic_hint() {
        for code in ExitCode::all() {
            let hint = code.hint(&ErrorContext::empty());
            if code.is_success() {
                assert!(hint.is_empty(), "Success must not carry a hint");
            } else {
                assert!(
                    !hint.is_empty(),
                    "{} must carry a recovery hint per the CLI Standard's tips-thinking",
                    code.name()
                );
            }
        }
    }

    #[test]
    fn not_found_hint_interpolates_query_when_provided() {
        let with_query = ExitCode::NotFound.hint(&ErrorContext::with_query("widget"));
        assert!(with_query.contains("widget"), "got {with_query}");
        assert!(with_query.contains("loran search"));

        let without = ExitCode::NotFound.hint(&ErrorContext::empty());
        assert!(without.contains("loran list"));
    }

    #[test]
    fn live_help_timeout_hint_uses_tool_name_when_provided() {
        let with_tool = ExitCode::LiveHelpTimeout.hint(&ErrorContext::with_tool("widgetctl"));
        assert!(with_tool.contains("widgetctl"));
        assert!(with_tool.starts_with("loran new"));
    }

    #[test]
    fn overlay_write_denied_hint_uses_tool_name_when_provided() {
        let h = ExitCode::OverlayWriteDenied.hint(&ErrorContext::with_tool("eza"));
        assert!(h.contains("mkdir -p"));
        assert!(h.contains("eza"));
    }

    #[test]
    fn index_not_built_hint_is_exact_match_for_spec_table() {
        assert_eq!(
            ExitCode::IndexNotBuilt.hint(&ErrorContext::empty()),
            "loran update"
        );
    }

    #[test]
    fn tarball_verify_failed_hint_includes_publisher_key_caveat() {
        let h = ExitCode::TarballVerifyFailed.hint(&ErrorContext::empty());
        assert!(h.contains("loran update"));
        assert!(h.contains("publisher key"));
    }

    #[test]
    fn display_emits_name_and_numeric() {
        assert_eq!(ExitCode::NotFound.to_string(), "NOT_FOUND (3)");
        assert_eq!(ExitCode::Success.to_string(), "SUCCESS (0)");
    }

    #[test]
    fn is_success_only_matches_success() {
        for code in ExitCode::all() {
            assert_eq!(code.is_success(), matches!(code, ExitCode::Success));
        }
    }
}
