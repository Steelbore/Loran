// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Error types for ingestion and indexing.

use std::io;

use loran_pages::PageError;
use thiserror::Error;

/// Errors raised by an [`crate::Ingestor`] while producing pages.
#[derive(Debug, Error)]
pub enum IngestError {
    /// Filesystem or other I/O failure while reading source data.
    ///
    /// The [`MarkdownPagesIngestor`] surfaces both directory-traversal
    /// errors (via `walkdir`) and individual `fs::read_to_string`
    /// failures through this variant.
    ///
    /// [`MarkdownPagesIngestor`]: crate::MarkdownPagesIngestor
    #[error("I/O error during ingestion: {0}")]
    Io(#[from] io::Error),

    /// A page failed schema validation.
    ///
    /// The `path` is the human-readable source identifier (a filesystem
    /// path for the bundled-pages ingester; the producing binary name
    /// for the future `DescribeIngestor`). It is intentionally a `String`
    /// so non-filesystem sources can describe themselves freely.
    #[error("page `{path}` failed to parse: {source}")]
    Page {
        /// Source identifier of the offending page.
        path: String,
        /// The underlying parse failure.
        #[source]
        source: PageError,
    },

    /// The ingestion source itself is malformed.
    ///
    /// Used when a source's overall shape is wrong before any individual
    /// page is parsed — e.g. the directory passed to
    /// [`MarkdownPagesIngestor`] does not exist.
    ///
    /// [`MarkdownPagesIngestor`]: crate::MarkdownPagesIngestor
    #[error("invalid ingestion source: {0}")]
    BadSource(String),
}

impl From<walkdir::Error> for IngestError {
    fn from(err: walkdir::Error) -> Self {
        // walkdir wraps the OS error in most cases; recover the inner
        // io::Error so callers see a consistent variant. Symbolic-link
        // cycle errors carry no io::Error — fall back to BadSource.
        let path = err.path().map(|p| p.display().to_string());
        match err.into_io_error() {
            Some(io_err) => IngestError::Io(io_err),
            None => IngestError::BadSource(path.unwrap_or_else(|| "<unknown path>".to_owned())),
        }
    }
}

/// Errors raised by [`crate::Index::build`].
#[derive(Debug, Error)]
pub enum IndexError {
    /// An [`crate::Ingestor`] failed before any indexing happened.
    #[error(transparent)]
    Ingest(#[from] IngestError),

    /// Two pages claim the same canonical [`loran_pages::Page::name`].
    ///
    /// Names must be globally unique within an index. Overlay merging
    /// (Phase 2) resolves precedence between overlapping pages from
    /// different sources; in v1 the bundled-pages ingester is the only
    /// source, so any duplicate is a content bug.
    #[error("duplicate page name `{0}` — names must be globally unique within an index")]
    DuplicateName(String),
}
