// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Three-layer overlay merge engine (WP-P2.13, Spec §5.1).
//!
//! Loran resolves pages by merging three roots in precedence order:
//!
//! 1. Upstream `pages/` (lowest)
//! 2. `overlays/<active-distro>/` (Bravais, Ferrite, or `generic`)
//! 3. `overlays/user/` (highest)
//!
//! A given on-disk page may be either a **full** page (every required
//! frontmatter field present) or a **partial** overlay (only `name`
//! required — see [`OverlayPage`](loran_pages::OverlayPage)). The
//! [`LayeredIngestor`] tries [`Page::parse`] first; if the only thing
//! missing is `category` or `summary`, it re-parses as an
//! [`OverlayPage`].
//!
//! Per-`name` rules:
//!
//! - First (lowest) layer must establish a complete [`Page`] — a partial
//!   overlay with no base is an error.
//! - Each subsequent layer either replaces the page wholesale (full
//!   page) or applies a partial overlay onto the running record.
//! - Missing layer directories are silently skipped.
//!
//! `walkdir`-based traversal mirrors [`crate::MarkdownPagesIngestor`]:
//! `.git/` and `.dotfiles` are skipped, non-`.md` files (e.g.
//! `categories.toml`) are ignored.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use loran_pages::{OverlayPage, Page, PageError};
use walkdir::WalkDir;

use crate::error::IngestError;
use crate::ingestor::Ingestor;

/// One layer in the overlay stack.
#[derive(Debug, Clone)]
pub struct OverlayLayer {
    /// Human-readable layer identifier used in error messages
    /// (`"upstream"`, `"bravais"`, `"user"`, …).
    pub name: String,
    /// Root directory for this layer. May be absent on disk; that case
    /// is silently skipped by the ingestor.
    pub root: PathBuf,
}

impl OverlayLayer {
    /// Convenience constructor.
    #[must_use]
    pub fn new(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            root: root.into(),
        }
    }
}

/// Ingester that walks an ordered list of [`OverlayLayer`]s and emits
/// one [`Page`] per canonical name.
///
/// Layers are processed in the order they're supplied: lowest
/// precedence first. The result of [`Ingestor::ingest`] is the merged
/// catalog.
#[derive(Debug, Clone)]
pub struct LayeredIngestor {
    layers: Vec<OverlayLayer>,
}

impl LayeredIngestor {
    /// Build a layered ingestor from `layers`, lowest-precedence first.
    #[must_use]
    pub fn new(layers: Vec<OverlayLayer>) -> Self {
        Self { layers }
    }

    /// Inspect the ordered layer list. Useful for diagnostics + tests.
    #[must_use]
    pub fn layers(&self) -> &[OverlayLayer] {
        &self.layers
    }
}

impl Ingestor for LayeredIngestor {
    fn ingest(&self) -> Result<Vec<Page>, IngestError> {
        let mut merged: HashMap<String, Page> = HashMap::new();

        for layer in &self.layers {
            ingest_layer(layer, &mut merged)?;
        }

        let mut out: Vec<Page> = merged.into_values().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

fn ingest_layer(
    layer: &OverlayLayer,
    merged: &mut HashMap<String, Page>,
) -> Result<(), IngestError> {
    if !layer.root.exists() {
        // Missing layer dir is normal (no user overlay yet, no distro
        // overlay shipped for `generic`, …). Spec §5.1: "Falls back to
        // the next layer".
        return Ok(());
    }
    if !layer.root.is_dir() {
        return Err(IngestError::BadSource(format!(
            "overlay `{}` root is not a directory: {}",
            layer.name,
            layer.root.display()
        )));
    }

    for entry in markdown_files(&layer.root) {
        let entry = entry?;
        let path = entry.path();
        let text = fs::read_to_string(path)?;
        apply_file(layer, path, &text, merged)?;
    }
    Ok(())
}

fn markdown_files(root: &Path) -> impl Iterator<Item = Result<walkdir::DirEntry, walkdir::Error>> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            entry
                .file_name()
                .to_str()
                .is_none_or(|n| !n.starts_with('.'))
        })
        .filter(|res| match res {
            Ok(entry) => {
                entry.file_type().is_file()
                    && entry.path().extension().and_then(|s| s.to_str()) == Some("md")
            }
            Err(_) => true,
        })
}

fn apply_file(
    layer: &OverlayLayer,
    path: &Path,
    text: &str,
    merged: &mut HashMap<String, Page>,
) -> Result<(), IngestError> {
    // Try the full-page parse first — that's the lowest-cost path and
    // covers the upstream layer's exclusive case.
    match Page::parse(text) {
        Ok(page) => {
            merged.insert(page.name.clone(), page);
            return Ok(());
        }
        Err(PageError::MissingField(field)) if field == "category" || field == "summary" => {
            // Could be a partial overlay — fall through.
        }
        Err(source) => {
            return Err(IngestError::Page {
                path: format!("[{}] {}", layer.name, path.display()),
                source,
            });
        }
    }

    let overlay = OverlayPage::parse(text).map_err(|source| IngestError::Page {
        path: format!("[{}] {}", layer.name, path.display()),
        source,
    })?;

    match merged.get(&overlay.name).cloned() {
        Some(base) => {
            let merged_page = base
                .merge_overlay(overlay)
                .map_err(|source| IngestError::Page {
                    path: format!("[{}] {}", layer.name, path.display()),
                    source,
                })?;
            merged.insert(merged_page.name.clone(), merged_page);
        }
        None => {
            // No lower-precedence base: a partial overlay needs one.
            // Surface as a parse error so the user sees which file is
            // dangling.
            return Err(IngestError::Page {
                path: format!("[{}] {}", layer.name, path.display()),
                source: PageError::MissingField(
                    "overlay file has no base in a lower-precedence layer; \
                     supply category + summary or remove the file",
                ),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use loran_pages::Page;
    use tempfile::TempDir;

    use super::{LayeredIngestor, OverlayLayer};
    use crate::Ingestor;
    use crate::error::IngestError;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    fn upstream_eza() -> &'static str {
        "+++\n\
         name = \"eza\"\n\
         category = \"file-listing\"\n\
         summary = \"Modern ls replacement.\"\n\
         replaces = [\"ls\"]\n\
         +++\n\
         \nUpstream body.\n"
    }

    fn find_page<'a>(pages: &'a [Page], name: &str) -> &'a Page {
        pages.iter().find(|p| p.name == name).expect("name present")
    }

    #[test]
    fn missing_layer_dirs_are_silently_skipped() {
        let upstream = TempDir::new().unwrap();
        write(upstream.path(), "eza.md", upstream_eza());

        let ingestor = LayeredIngestor::new(vec![
            OverlayLayer::new("upstream", upstream.path()),
            OverlayLayer::new(
                "bravais",
                Path::new("/does/not/exist/loran/overlays/bravais"),
            ),
            OverlayLayer::new(
                "user",
                Path::new("/also/does/not/exist/loran/overlays/user"),
            ),
        ]);
        let pages = ingestor.ingest().expect("missing dirs OK");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].name, "eza");
    }

    #[test]
    fn user_overlay_overrides_summary_only() {
        let upstream = TempDir::new().unwrap();
        let user = TempDir::new().unwrap();
        write(upstream.path(), "eza.md", upstream_eza());
        write(
            user.path(),
            "eza.md",
            "+++\nname = \"eza\"\nsummary = \"User-pinned ls.\"\n+++\n",
        );

        let pages = LayeredIngestor::new(vec![
            OverlayLayer::new("upstream", upstream.path()),
            OverlayLayer::new("user", user.path()),
        ])
        .ingest()
        .unwrap();
        let eza = find_page(&pages, "eza");
        assert_eq!(eza.summary, "User-pinned ls.");
        // Inherited:
        assert_eq!(eza.category, "file-listing");
        assert_eq!(eza.replaces, vec!["ls"]);
    }

    #[test]
    fn distro_layer_sits_between_upstream_and_user() {
        let upstream = TempDir::new().unwrap();
        let distro = TempDir::new().unwrap();
        let user = TempDir::new().unwrap();

        write(upstream.path(), "eza.md", upstream_eza());
        write(
            distro.path(),
            "eza.md",
            "+++\nname = \"eza\"\nsummary = \"Bravais ls.\"\nreplaces = [\"ls\", \"dir\"]\n+++\n",
        );
        write(
            user.path(),
            "eza.md",
            "+++\nname = \"eza\"\nsummary = \"User ls.\"\n+++\n",
        );

        let pages = LayeredIngestor::new(vec![
            OverlayLayer::new("upstream", upstream.path()),
            OverlayLayer::new("bravais", distro.path()),
            OverlayLayer::new("user", user.path()),
        ])
        .ingest()
        .unwrap();
        let eza = find_page(&pages, "eza");
        // User overlay wins on summary; distro's replaces survives.
        assert_eq!(eza.summary, "User ls.");
        assert_eq!(eza.replaces, vec!["ls", "dir"]);
    }

    #[test]
    fn higher_layer_can_introduce_new_full_page() {
        let upstream = TempDir::new().unwrap();
        let user = TempDir::new().unwrap();
        write(upstream.path(), "eza.md", upstream_eza());
        write(
            user.path(),
            "btop.md",
            "+++\n\
             name = \"btop\"\n\
             category = \"system-monitoring\"\n\
             summary = \"Resource monitor.\"\n\
             +++\n",
        );

        let pages = LayeredIngestor::new(vec![
            OverlayLayer::new("upstream", upstream.path()),
            OverlayLayer::new("user", user.path()),
        ])
        .ingest()
        .unwrap();
        assert_eq!(pages.len(), 2);
        find_page(&pages, "btop");
        find_page(&pages, "eza");
    }

    #[test]
    fn partial_overlay_without_base_fails_loud() {
        let upstream = TempDir::new().unwrap();
        let user = TempDir::new().unwrap();
        // No "eza" upstream; the overlay is dangling.
        write(
            user.path(),
            "eza.md",
            "+++\nname = \"eza\"\nsummary = \"floating.\"\n+++\n",
        );

        let err = LayeredIngestor::new(vec![
            OverlayLayer::new("upstream", upstream.path()),
            OverlayLayer::new("user", user.path()),
        ])
        .ingest()
        .unwrap_err();

        match err {
            IngestError::Page { path, .. } => {
                assert!(
                    path.contains("[user]"),
                    "error must identify the offending layer; got {path}"
                );
            }
            other => panic!("expected IngestError::Page, got {other:?}"),
        }
    }

    #[test]
    fn malformed_page_in_overlay_reports_layer_in_path() {
        let upstream = TempDir::new().unwrap();
        write(
            upstream.path(),
            "broken.md",
            "+++\nname = \"x\"\nrepleaces = []\n+++\n", // typo: repleaces
        );
        let err = LayeredIngestor::new(vec![OverlayLayer::new("upstream", upstream.path())])
            .ingest()
            .unwrap_err();
        match err {
            IngestError::Page { path, .. } => assert!(path.contains("[upstream]")),
            other => panic!("expected IngestError::Page, got {other:?}"),
        }
    }

    #[test]
    fn non_md_files_under_layer_root_are_ignored() {
        let upstream = TempDir::new().unwrap();
        write(upstream.path(), "eza.md", upstream_eza());
        write(upstream.path(), "categories.toml", "[file-listing]\n");
        write(upstream.path(), ".hidden/notes.md", "not a page");
        let pages = LayeredIngestor::new(vec![OverlayLayer::new("upstream", upstream.path())])
            .ingest()
            .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].name, "eza");
    }
}
