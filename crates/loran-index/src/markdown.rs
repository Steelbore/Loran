// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! [`MarkdownPagesIngestor`] — walks a directory tree of `*.md` files.

use std::fs;
use std::path::{Path, PathBuf};

use loran_pages::Page;
use walkdir::WalkDir;

use crate::error::IngestError;
use crate::ingestor::Ingestor;

/// Ingests every `*.md` file under a root directory as a Loran page.
///
/// Walks the directory recursively. Hidden entries (filename starting
/// with `.`) are skipped, so a `.git/` directory dropped under the
/// pages root is silently ignored. Non-`.md` files (e.g. the
/// `categories.toml` that lives alongside pages in the upstream
/// catalog) are skipped too.
///
/// Every discovered `.md` file is read with [`fs::read_to_string`] and
/// parsed with [`Page::parse`]. The first failure aborts the walk with
/// [`IngestError::Page`] or [`IngestError::Io`]; pages already parsed
/// successfully are discarded.
#[derive(Debug, Clone)]
pub struct MarkdownPagesIngestor {
    root: PathBuf,
}

impl MarkdownPagesIngestor {
    /// Construct an ingester rooted at `root`.
    ///
    /// The directory is **not** validated here; failure to open it is
    /// surfaced from [`Ingestor::ingest`] as [`IngestError::BadSource`].
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Return the root directory passed at construction time.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Ingestor for MarkdownPagesIngestor {
    fn ingest(&self) -> Result<Vec<Page>, IngestError> {
        if !self.root.exists() {
            return Err(IngestError::BadSource(format!(
                "root does not exist: {}",
                self.root.display()
            )));
        }
        if !self.root.is_dir() {
            return Err(IngestError::BadSource(format!(
                "root is not a directory: {}",
                self.root.display()
            )));
        }

        let mut pages = Vec::new();

        let walker = WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                // Skip hidden entries (.git, .DS_Store, .anything) but
                // never skip the root itself even if `root` happens to
                // start with '.'.
                if entry.depth() == 0 {
                    return true;
                }
                entry
                    .file_name()
                    .to_str()
                    .is_none_or(|n| !n.starts_with('.'))
            });

        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let body = fs::read_to_string(path)?;
            match Page::parse(&body) {
                Ok(page) => pages.push(page),
                Err(source) => {
                    return Err(IngestError::Page {
                        path: path.display().to_string(),
                        source,
                    });
                }
            }
        }

        Ok(pages)
    }
}
