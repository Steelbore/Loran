// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! tldr-pages cache + lookup.
//!
//! Phase 2 (`WP-P2.11` / `WP-P2.17`) ships the read-side surface over
//! the tldr-pages archive that `loran update` installs under
//! `$XDG_CACHE_HOME/loran/tldr/extracted/`. After a successful
//! `update_tldr` run the directory looks like:
//!
//! ```text
//! $XDG_CACHE_HOME/loran/tldr/extracted/
//! └── pages/
//!     ├── common/<tool>.md
//!     ├── linux/<tool>.md
//!     ├── osx/<tool>.md
//!     ├── windows/<tool>.md
//!     └── freebsd/<tool>.md
//! ```
//!
//! v1 resolution order: `common` → `linux` → `osx` → `windows` →
//! `freebsd`. First file that exists wins. A future
//! `--tldr-platform=<list>` flag can re-order. Localised page
//! directories (`pages.<lang>/`) are reserved for the Spec §6.1
//! i18n carve-out and are not consulted by v1.

use std::fs;
use std::path::{Path, PathBuf};

/// Default platform-fallback order. Conservative — `common` first,
/// host-native second, the rest in arbitrary published-source order.
pub const DEFAULT_PLATFORMS: &[&str] = &["common", "linux", "osx", "windows", "freebsd"];

/// Read-side handle to the tldr-pages extracted tree.
#[derive(Debug, Clone)]
pub struct TldrCache {
    extract_dir: PathBuf,
}

impl TldrCache {
    /// Resolve the canonical extracted-tree path under
    /// `$XDG_CACHE_HOME/loran/tldr/extracted/`. Returns `None` when no
    /// cache directory can be located.
    #[must_use]
    pub fn with_default_path() -> Option<Self> {
        let dir = crate::xdg::cache_home()?;
        Some(Self::with_path(
            dir.join("loran").join("tldr").join("extracted"),
        ))
    }

    /// Override the extracted-tree root. Used by tests and by
    /// containerised deployments where `$XDG_CACHE_HOME` is non-default.
    #[must_use]
    pub fn with_path(extract_dir: impl Into<PathBuf>) -> Self {
        Self {
            extract_dir: extract_dir.into(),
        }
    }

    /// Filesystem root of the extracted tree.
    #[must_use]
    pub fn extract_dir(&self) -> &Path {
        &self.extract_dir
    }

    /// Resolve a `<tool>.md` lookup in platform-fallback order.
    /// Returns the page body markdown on the first hit, `None`
    /// otherwise.
    #[must_use]
    pub fn lookup(&self, tool: &str) -> Option<String> {
        self.lookup_with_platforms(tool, DEFAULT_PLATFORMS)
    }

    /// Like [`Self::lookup`] but caller-controlled platform order.
    /// Mostly for tests; future `--tldr-platform` will wire here.
    #[must_use]
    pub fn lookup_with_platforms(&self, tool: &str, platforms: &[&str]) -> Option<String> {
        let safe = tool.trim();
        if safe.is_empty() || safe.contains('/') || safe.contains('\\') {
            return None;
        }
        for platform in platforms {
            let path = self
                .extract_dir
                .join("pages")
                .join(platform)
                .join(format!("{safe}.md"));
            if let Ok(body) = fs::read_to_string(&path) {
                return Some(body);
            }
        }
        None
    }
}

/// Pluggable lookup surface so [`crate::resolve_show_with_tldr`] can
/// be tested against a canned implementation without touching the real
/// filesystem.
pub trait TldrLookup {
    /// Resolve `tool` to its tldr body markdown, or `None`.
    fn lookup(&self, tool: &str) -> Option<String>;
}

impl TldrLookup for TldrCache {
    fn lookup(&self, tool: &str) -> Option<String> {
        TldrCache::lookup(self, tool)
    }
}

/// The "no tldr available" stub. Used by callers that don't have a
/// cache directory (the bare [`crate::resolve_show`]). Always returns
/// `None`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoTldr;

impl TldrLookup for NoTldr {
    fn lookup(&self, _tool: &str) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{NoTldr, TldrCache, TldrLookup};

    fn seed_cache(root: &std::path::Path, entries: &[(&str, &str, &str)]) {
        for (platform, tool, body) in entries {
            let dir = root.join("pages").join(platform);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("{tool}.md")), body).unwrap();
        }
    }

    #[test]
    fn lookup_returns_body_on_hit() {
        let dir = tempdir().unwrap();
        seed_cache(dir.path(), &[("common", "cat", "# cat\n\nConcatenate.\n")]);
        let cache = TldrCache::with_path(dir.path());
        let body = cache.lookup("cat").expect("hit");
        assert!(body.contains("Concatenate"));
    }

    #[test]
    fn lookup_returns_none_for_unknown_tool() {
        let dir = tempdir().unwrap();
        let cache = TldrCache::with_path(dir.path());
        assert!(cache.lookup("nope").is_none());
    }

    #[test]
    fn lookup_prefers_common_over_linux() {
        let dir = tempdir().unwrap();
        seed_cache(
            dir.path(),
            &[("common", "cat", "# common"), ("linux", "cat", "# linux")],
        );
        let cache = TldrCache::with_path(dir.path());
        let body = cache.lookup("cat").expect("hit");
        assert!(
            body.contains("common"),
            "common must win over linux; got {body}"
        );
    }

    #[test]
    fn lookup_falls_through_to_linux_when_only_linux_exists() {
        let dir = tempdir().unwrap();
        seed_cache(dir.path(), &[("linux", "procs", "# procs (linux)\n")]);
        let cache = TldrCache::with_path(dir.path());
        let body = cache.lookup("procs").expect("hit");
        assert!(body.contains("linux"));
    }

    #[test]
    fn path_like_names_are_rejected() {
        let dir = tempdir().unwrap();
        let cache = TldrCache::with_path(dir.path());
        assert!(cache.lookup("../etc/passwd").is_none());
        assert!(cache.lookup("a/b").is_none());
        assert!(cache.lookup("").is_none());
    }

    #[test]
    fn no_tldr_stub_always_returns_none() {
        let stub = NoTldr;
        assert!(stub.lookup("eza").is_none());
        // Also exercise as &dyn TldrLookup for the trait-object path.
        let as_dyn: &dyn TldrLookup = &stub;
        assert!(as_dyn.lookup("anything").is_none());
    }

    #[test]
    fn lookup_with_custom_platform_order() {
        let dir = tempdir().unwrap();
        seed_cache(
            dir.path(),
            &[("common", "cat", "# common"), ("osx", "cat", "# osx")],
        );
        let cache = TldrCache::with_path(dir.path());
        // Custom order: osx wins when listed first.
        let body = cache
            .lookup_with_platforms("cat", &["osx", "common"])
            .expect("hit");
        assert!(body.contains("osx"));
    }
}
