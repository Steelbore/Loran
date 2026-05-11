// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! [`BundledPagesIngestor`] — read-side surface over the pages baked
//! into the `loran` binary at compile time.
//!
//! The build script ([`crates/loran-core/build.rs`]) walks the
//! workspace-root `pages/` directory at compile time, validates every
//! file via [`loran_pages::Page::parse`], and emits a generated
//! `BUNDLED_PAGES: &[(&str, &str)]` constant containing `include_str!`
//! references. This module wraps that constant in an [`Ingestor`]
//! implementation so the bundled catalog flows through the same
//! pipeline as filesystem and (Phase 3) `describe`-derived sources.

use loran_index::{IngestError, Ingestor};
use loran_pages::Page;

use crate::bundled_data::BUNDLED_PAGES;

/// Read-only [`Ingestor`] backed by the build-time bundle.
#[derive(Debug, Default, Clone, Copy)]
pub struct BundledPagesIngestor;

impl BundledPagesIngestor {
    /// Construct an ingester. Equivalent to [`Self::default`].
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Number of pages baked into the binary at compile time.
    ///
    /// Useful for sanity checks (`assert!(BundledPagesIngestor::len() >= 8)`)
    /// in callers that want to fail loudly if a future workspace
    /// refactor accidentally empties the bundle.
    #[must_use]
    pub const fn len() -> usize {
        BUNDLED_PAGES.len()
    }

    /// `true` when the bundle is empty. Always `false` for production
    /// builds — `loran-core/build.rs` panics if `pages/` is empty.
    #[must_use]
    #[allow(clippy::const_is_empty)] // emptiness depends on the generated file
    pub const fn is_empty() -> bool {
        BUNDLED_PAGES.is_empty()
    }

    /// Iterator over the raw `(relative_path, body_md)` tuples.
    ///
    /// Pre-parse access mostly exists for tooling — `loran validate`
    /// and the future `loran schema` paths use the raw form to
    /// re-validate or to compute structural metrics.
    pub fn entries() -> impl Iterator<Item = (&'static str, &'static str)> {
        BUNDLED_PAGES.iter().copied()
    }
}

impl Ingestor for BundledPagesIngestor {
    fn ingest(&self) -> Result<Vec<Page>, IngestError> {
        let mut pages = Vec::with_capacity(BUNDLED_PAGES.len());
        for (path, body) in BUNDLED_PAGES {
            let page = Page::parse(body).map_err(|source| IngestError::Page {
                path: format!("bundled:{path}"),
                source,
            })?;
            pages.push(page);
        }
        Ok(pages)
    }
}

#[cfg(test)]
mod tests {
    use loran_index::{Index, Ingestor};

    use super::BundledPagesIngestor;

    #[test]
    fn bundle_contains_at_least_one_page() {
        assert!(
            !BundledPagesIngestor::is_empty(),
            "bundled catalog must not ship empty"
        );
        assert!(
            BundledPagesIngestor::len() >= 1,
            "bundled catalog has {} pages; seed minimum is 1",
            BundledPagesIngestor::len()
        );
    }

    #[test]
    fn every_bundled_page_parses_cleanly() {
        let pages = BundledPagesIngestor::new()
            .ingest()
            .expect("bundled pages must parse — build.rs already validated");
        assert_eq!(pages.len(), BundledPagesIngestor::len());
    }

    #[test]
    fn bundled_pages_build_a_valid_index() {
        let pages = BundledPagesIngestor::new().ingest().expect("bundle parses");
        let idx = Index::build(pages).expect("index builds without duplicates");
        assert_eq!(idx.len(), BundledPagesIngestor::len());
    }

    #[test]
    fn bundle_covers_multiple_categories() {
        let pages = BundledPagesIngestor::new().ingest().unwrap();
        let idx = Index::build(pages).unwrap();
        let cats: Vec<&str> = idx.categories().collect();
        assert!(
            cats.len() >= 3,
            "bundled catalog should span ≥3 categories; got {cats:?}"
        );
    }

    #[test]
    fn seed_includes_the_canonical_eza_entry() {
        // Sanity: if `eza` ever falls out of the bundle, the rest of
        // the documentation that points at it as the canonical example
        // will go stale.
        let pages = BundledPagesIngestor::new().ingest().unwrap();
        let idx = Index::build(pages).unwrap();
        let eza = idx.get("eza").expect("eza must remain in the bundle");
        assert_eq!(eza.category, "file-listing");
        assert!(eza.replaces.iter().any(|r| r == "ls"));
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)] // we control these paths
    fn entries_paths_are_relative_and_normalised() {
        for (path, _body) in BundledPagesIngestor::entries() {
            assert!(
                !path.starts_with('/'),
                "bundled entry path `{path}` must be relative to the pages/ root"
            );
            assert!(
                !path.contains('\\'),
                "bundled entry path `{path}` must use forward slashes"
            );
            assert!(
                path.ends_with(".md"),
                "bundled entry path `{path}` must end with .md"
            );
        }
    }
}
