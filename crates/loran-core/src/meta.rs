// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Per-source metadata cache for `loran update`.
//!
//! Loran needs to remember, between invocations:
//!
//! - The HTTP `ETag` returned with the last successful tarball fetch
//!   (so the next call can short-circuit on `304 Not Modified`).
//! - The catalog version we last installed (for human-readable status
//!   and for staleness detection).
//! - When we last fetched (ISO 8601 UTC).
//!
//! The persisted file lives at `$XDG_CACHE_HOME/loran/sources.toml`.
//! Multiple sources (`upstream-pages`, `tldr-pages`, future overlay
//! catalogs) share one TOML file keyed by source name.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Filename written under the user's cache directory.
const META_FILENAME: &str = "sources.toml";

/// Source-meta record. One per upstream tarball source.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SourceMeta {
    /// HTTP `ETag` from the last successful manifest fetch. Passed as
    /// `If-None-Match` on the next call so an unchanged catalog
    /// short-circuits to a 304 round trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,

    /// Catalog version (from the manifest) that we currently have
    /// installed. `None` means we've never successfully installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// When the last successful install / refresh completed.
    /// Serialised as ISO 8601 UTC with the `Z` suffix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<Timestamp>,
}

/// On-disk envelope: a map of source-name → [`SourceMeta`].
///
/// The file holds every source we know about, so reads / writes are
/// whole-document for simplicity. The catalog is small enough that this
/// is a non-issue.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceMetaFile {
    #[serde(flatten)]
    pub sources: BTreeMap<String, SourceMeta>,
}

#[derive(Debug, Error)]
pub enum MetaError {
    #[error("I/O error accessing source meta: {0}")]
    Io(#[from] std::io::Error),

    #[error("source meta file is not valid TOML: {0}")]
    Decode(#[from] toml::de::Error),

    #[error("source meta could not be serialised to TOML: {0}")]
    Encode(#[from] toml::ser::Error),

    #[error("no cache directory available on this platform")]
    NoCacheDir,
}

/// Handle to the sources meta file under `$XDG_CACHE_HOME/loran/`.
#[derive(Debug, Clone)]
pub struct SourceMetaStore {
    path: PathBuf,
}

impl SourceMetaStore {
    /// Resolve the canonical path: `$XDG_CACHE_HOME/loran/sources.toml`.
    pub fn with_default_path() -> Result<Self, MetaError> {
        let dir = crate::xdg::cache_home().ok_or(MetaError::NoCacheDir)?;
        Ok(Self::with_path(dir.join("loran").join(META_FILENAME)))
    }

    /// Override the file path. Used by tests, by `--meta-path <path>`
    /// (future), and by containerised deployments.
    #[must_use]
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Filesystem path the store reads / writes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the meta file. Returns an empty [`SourceMetaFile`] when
    /// the file does not yet exist (first-run case).
    pub fn load(&self) -> Result<SourceMetaFile, MetaError> {
        let bytes = match fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SourceMetaFile::default());
            }
            Err(e) => return Err(MetaError::Io(e)),
        };
        Ok(toml::from_str(&bytes)?)
    }

    /// Atomically write the meta file.
    pub fn save(&self, file: &SourceMetaFile) -> Result<(), MetaError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(file)?;
        let tmp = self.path.with_extension("toml.tmp");
        fs::write(&tmp, serialized.as_bytes())?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Convenience: read, mutate one source's record, write back.
    pub fn update_source<F: FnOnce(&mut SourceMeta)>(
        &self,
        source: &str,
        mutate: F,
    ) -> Result<SourceMetaFile, MetaError> {
        let mut file = self.load()?;
        let entry = file.sources.entry(source.to_owned()).or_default();
        mutate(entry);
        self.save(&file)?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use tempfile::tempdir;

    use super::{SourceMeta, SourceMetaFile, SourceMetaStore};

    #[test]
    fn load_returns_empty_when_file_missing() {
        let dir = tempdir().unwrap();
        let store = SourceMetaStore::with_path(dir.path().join("sources.toml"));
        let file = store.load().expect("load");
        assert!(file.sources.is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempdir().unwrap();
        let store = SourceMetaStore::with_path(dir.path().join("sources.toml"));

        let original = SourceMetaFile {
            sources: [(
                "upstream-pages".to_owned(),
                SourceMeta {
                    etag: Some("\"abc-def\"".to_owned()),
                    version: Some("v1.2.3".to_owned()),
                    fetched_at: Some(Timestamp::from_second(1_716_000_000).unwrap()),
                },
            )]
            .into_iter()
            .collect(),
        };

        store.save(&original).expect("save");
        let loaded = store.load().expect("load");
        assert_eq!(loaded, original);
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = tempdir().unwrap();
        let path = dir
            .path()
            .join("deeply")
            .join("nested")
            .join("sources.toml");
        let store = SourceMetaStore::with_path(&path);
        store
            .save(&SourceMetaFile::default())
            .expect("creates parent");
        assert!(path.exists());
    }

    #[test]
    fn update_source_creates_then_mutates() {
        let dir = tempdir().unwrap();
        let store = SourceMetaStore::with_path(dir.path().join("sources.toml"));

        // First call creates the entry.
        let file = store
            .update_source("upstream-pages", |m| {
                m.etag = Some("e1".into());
                m.version = Some("v0.1".into());
            })
            .expect("create");
        assert_eq!(
            file.sources
                .get("upstream-pages")
                .and_then(|m| m.etag.as_deref()),
            Some("e1")
        );

        // Second call mutates.
        let file = store
            .update_source("upstream-pages", |m| {
                m.etag = Some("e2".into());
            })
            .expect("mutate");
        assert_eq!(
            file.sources
                .get("upstream-pages")
                .and_then(|m| m.etag.as_deref()),
            Some("e2")
        );
        // Version preserved across the targeted mutation.
        assert_eq!(
            file.sources
                .get("upstream-pages")
                .and_then(|m| m.version.as_deref()),
            Some("v0.1")
        );
    }

    #[test]
    fn empty_options_are_omitted_from_serialised_form() {
        let dir = tempdir().unwrap();
        let store = SourceMetaStore::with_path(dir.path().join("sources.toml"));
        let original = SourceMetaFile {
            sources: [(
                "tldr-pages".to_owned(),
                SourceMeta {
                    etag: None,
                    version: None,
                    fetched_at: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        store.save(&original).expect("save");
        let body = std::fs::read_to_string(store.path()).unwrap();
        assert!(
            !body.contains("etag"),
            "None fields must not serialise: {body}"
        );
        assert!(!body.contains("version"));
        assert!(!body.contains("fetched_at"));
    }
}
