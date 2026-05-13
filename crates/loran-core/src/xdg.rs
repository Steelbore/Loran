// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Cross-platform XDG base-directory resolution.
//!
//! The `dirs` crate follows each platform's *native* convention —
//! `~/.local/share` on Linux, `~/Library/Application Support` on
//! macOS, `%APPDATA%` on Windows — and **does not consult
//! `$XDG_DATA_HOME` / `$XDG_CACHE_HOME` on macOS**. That breaks
//! integration tests that set those vars to a per-test tempdir to
//! sandbox writes: macOS happily ignores the override and the test
//! pollutes the runner's real home.
//!
//! This module makes the XDG environment variables authoritative
//! across every platform. When unset, it falls back to `dirs::*`,
//! preserving the native convention for end users who haven't
//! explicitly opted into XDG semantics.

use std::path::PathBuf;

/// Resolve the base data-home directory (where Loran installs its
/// `pages/`, `overlays/`, and `templates/` trees).
///
/// Order of precedence:
/// 1. `$XDG_DATA_HOME` (any platform, when non-empty).
/// 2. `dirs::data_dir()` (native convention per platform).
#[must_use]
pub fn data_home() -> Option<PathBuf> {
    env_path("XDG_DATA_HOME").or_else(dirs::data_dir)
}

/// Resolve the base cache-home directory.
///
/// Order of precedence:
/// 1. `$XDG_CACHE_HOME` (any platform, when non-empty).
/// 2. `dirs::cache_dir()` (native convention per platform).
#[must_use]
pub fn cache_home() -> Option<PathBuf> {
    env_path("XDG_CACHE_HOME").or_else(dirs::cache_dir)
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

// Integration coverage lives in the per-verb tests under
// `crates/loran/tests/`, which set `XDG_DATA_HOME` /
// `XDG_CACHE_HOME` to per-test tempdirs and assert the resulting
// path on every platform in the CI matrix. Adding unit tests here
// would require `unsafe { std::env::set_var(..) }` which this crate
// forbids; the integration suite already verifies the contract
// hermetically.
