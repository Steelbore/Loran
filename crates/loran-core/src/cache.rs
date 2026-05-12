// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Postcard binary cache for [`loran_index::Index`].
//!
//! Phase 2 (Billet) writes a serialised snapshot of the merged catalog
//! index to `$XDG_CACHE_HOME/loran/index.postcard`. Cold startup loads
//! the cache directly instead of re-walking the bundled pages tree and
//! re-parsing every page, satisfying PRD NFR-001 (sub-50 ms cold
//! `loran show`).
//!
//! The cache file format is `postcard` — a compact, schema-stable
//! binary serialisation. A format version is prepended so future
//! schema changes can invalidate the cache cleanly.
//!
//! The cache is **not** authoritative: callers that detect a missing,
//! stale, or unreadable cache fall back to a fresh rebuild from an
//! [`Ingestor`]. The `loran update` sub-command (Phase 2 [`WP-P2.12`])
//! is the canonical refresh trigger.
//!
//! [`Ingestor`]: loran_index::Ingestor

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use loran_index::Index;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current cache file format version. Bump when the on-disk layout
/// changes incompatibly; existing caches with a different version are
/// silently ignored and rebuilt.
pub const CACHE_FORMAT_VERSION: u32 = 1;

/// Default cache filename under the resolved cache directory.
const CACHE_FILENAME: &str = "index.postcard";

/// Errors raised by [`Cache::load`] / [`Cache::save`].
#[derive(Debug, Error)]
pub enum CacheError {
    /// Filesystem error while reading or writing the cache.
    #[error("cache I/O error: {0}")]
    Io(#[from] io::Error),

    /// The cache file exists but is malformed (truncated, corrupt, or
    /// written by an incompatible Loran version).
    #[error("cache file is malformed: {0}")]
    Corrupt(String),

    /// The cache file was written by a different format version.
    #[error(
        "cache format version mismatch (got {found}, expected {expected}); \
         caller should rebuild via `loran update`"
    )]
    VersionMismatch { found: u32, expected: u32 },

    /// `$XDG_CACHE_HOME` (or the platform equivalent) is unavailable.
    #[error("no cache directory available on this platform")]
    NoCacheDir,
}

/// Wire-level envelope written to disk. Wraps the serialised
/// [`Index`] with a version tag so version-bump invalidation is free.
#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    index: Index,
}

/// Read-side / write-side handle to the postcard cache.
#[derive(Debug, Clone)]
pub struct Cache {
    path: PathBuf,
}

impl Cache {
    /// Construct a `Cache` rooted at the platform default cache dir
    /// (`$XDG_CACHE_HOME/loran/` on Linux, equivalent on macOS /
    /// Windows). Returns [`CacheError::NoCacheDir`] when no cache
    /// directory can be resolved.
    pub fn with_default_path() -> Result<Self, CacheError> {
        let dir = dirs::cache_dir().ok_or(CacheError::NoCacheDir)?;
        Ok(Self::with_path(dir.join("loran").join(CACHE_FILENAME)))
    }

    /// Construct a `Cache` writing to an explicit file path. Used by
    /// tests, by `--cache-path <path>` overrides (Phase 2), and by
    /// containerised deployments where `$XDG_CACHE_HOME` is set
    /// non-conventionally.
    #[must_use]
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Filesystem path the cache reads from / writes to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load a cached [`Index`].
    ///
    /// Returns `Ok(None)` when the cache file does not exist (a normal
    /// cold-startup case — caller should fall back to a fresh build).
    /// `Ok(Some(index))` when the cache loaded successfully.
    /// `Err(_)` for I/O failures, malformed cache, or version mismatch;
    /// callers may treat any error as a hint to rebuild.
    pub fn load(&self) -> Result<Option<Index>, CacheError> {
        let bytes = match fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(CacheError::Io(e)),
        };
        let file: CacheFile =
            postcard::from_bytes(&bytes).map_err(|e| CacheError::Corrupt(e.to_string()))?;
        if file.version != CACHE_FORMAT_VERSION {
            return Err(CacheError::VersionMismatch {
                found: file.version,
                expected: CACHE_FORMAT_VERSION,
            });
        }
        Ok(Some(file.index))
    }

    /// Atomically write `index` to the cache.
    ///
    /// Writes to `<path>.tmp` first and renames to `<path>` so a
    /// concurrent reader either sees the previous version or the new
    /// version — never a torn write.
    pub fn save(&self, index: &Index) -> Result<(), CacheError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = CacheFile {
            version: CACHE_FORMAT_VERSION,
            index: index.clone(),
        };
        let bytes = postcard::to_allocvec(&file).map_err(|e| CacheError::Corrupt(e.to_string()))?;
        let tmp = self.path.with_extension("postcard.tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use loran_pages::Page;
    use tempfile::tempdir;

    use super::{CACHE_FORMAT_VERSION, Cache, CacheError, CacheFile};
    use crate::BundledPagesIngestor;
    use loran_index::{Index, Ingestor};

    fn build_bundled_index() -> Index {
        let pages = BundledPagesIngestor::new().ingest().expect("ingest");
        Index::build(pages).expect("build")
    }

    #[test]
    fn load_returns_none_when_cache_missing() {
        let dir = tempdir().unwrap();
        let cache = Cache::with_path(dir.path().join("does-not-exist.postcard"));
        assert!(cache.load().expect("ok").is_none());
    }

    #[test]
    fn save_and_load_round_trips_bundled_index() {
        let dir = tempdir().unwrap();
        let cache = Cache::with_path(dir.path().join("index.postcard"));
        let original = build_bundled_index();

        cache.save(&original).expect("save");
        let loaded = cache.load().expect("load").expect("present");

        assert_eq!(loaded.len(), original.len());

        // Round-trip preserves bodies — the whole point of removing the
        // serde-skip on Page::body in this WP.
        let eza_original = original.get("eza").expect("eza in bundle");
        let eza_loaded = loaded.get("eza").expect("eza in cache");
        assert_eq!(eza_loaded.name, eza_original.name);
        assert!(
            !eza_loaded.body.is_empty(),
            "eza body should round-trip through the cache"
        );
        assert_eq!(eza_loaded.body, eza_original.body);
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let nested = dir
            .path()
            .join("nested")
            .join("more")
            .join("index.postcard");
        let cache = Cache::with_path(&nested);
        let idx = build_bundled_index();
        cache.save(&idx).expect("save creates parents");
        assert!(nested.exists());
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)] // we control the temp paths
    fn save_is_atomic_no_tmp_file_left_behind() {
        let dir = tempdir().unwrap();
        let cache = Cache::with_path(dir.path().join("index.postcard"));
        cache.save(&build_bundled_index()).expect("save");

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "atomic rename should leave no .tmp behind: {leftovers:?}"
        );
    }

    #[test]
    fn load_rejects_corrupt_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("index.postcard");
        std::fs::write(&path, b"this is not valid postcard data").unwrap();
        let cache = Cache::with_path(&path);
        let err = cache.load().unwrap_err();
        assert!(matches!(err, CacheError::Corrupt(_)), "got {err:?}");
    }

    #[test]
    fn load_rejects_wrong_format_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("index.postcard");
        // Hand-write a CacheFile with a bogus version.
        let bogus = CacheFile {
            version: CACHE_FORMAT_VERSION + 99,
            index: Index::default(),
        };
        let bytes = postcard::to_allocvec(&bogus).unwrap();
        std::fs::write(&path, &bytes).unwrap();

        let cache = Cache::with_path(&path);
        let err = cache.load().unwrap_err();
        match err {
            CacheError::VersionMismatch { found, expected } => {
                assert_eq!(expected, CACHE_FORMAT_VERSION);
                assert_eq!(found, CACHE_FORMAT_VERSION + 99);
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn empty_index_round_trips() {
        let dir = tempdir().unwrap();
        let cache = Cache::with_path(dir.path().join("index.postcard"));
        let empty = Index::build(Vec::<Page>::new()).expect("empty build");
        cache.save(&empty).expect("save empty");
        let loaded = cache.load().expect("load").expect("present");
        assert!(loaded.is_empty());
    }
}
