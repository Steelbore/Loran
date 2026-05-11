// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Integration tests for [`MarkdownPagesIngestor`] against checked-in
//! fixture directory trees under `tests/fixtures/`.

use std::path::PathBuf;

use loran_index::{IngestError, Ingestor, MarkdownPagesIngestor};

/// Resolve `tests/fixtures/<subpath>` relative to the crate manifest.
fn fixture(subpath: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(subpath)
}

#[test]
fn ingests_three_valid_pages_from_nested_directories() {
    let ingestor = MarkdownPagesIngestor::new(fixture("pages"));
    let mut pages = ingestor.ingest().expect("valid fixture tree must ingest");
    pages.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(pages.len(), 3);
    assert_eq!(pages[0].name, "bat");
    assert_eq!(pages[1].name, "eza");
    assert_eq!(pages[2].name, "rg");

    // Categories survive directory layout — the ingester does not infer
    // category from the parent directory name; that's the page's own
    // frontmatter responsibility.
    assert_eq!(pages[0].category, "file-viewing");
    assert_eq!(pages[1].category, "file-listing");
    assert_eq!(pages[2].category, "text-search");
}

#[test]
fn empty_directory_yields_no_pages() {
    let ingestor = MarkdownPagesIngestor::new(fixture("empty"));
    let pages = ingestor.ingest().expect("empty dir is not an error");
    assert!(pages.is_empty());
}

#[test]
fn missing_root_returns_bad_source() {
    let ingestor = MarkdownPagesIngestor::new(fixture("does-not-exist"));
    let err = ingestor.ingest().unwrap_err();
    assert!(matches!(err, IngestError::BadSource(_)), "got {err:?}");
}

#[test]
fn malformed_page_surfaces_as_page_error_with_path() {
    let ingestor = MarkdownPagesIngestor::new(fixture("bad"));
    let err = ingestor.ingest().unwrap_err();
    match err {
        IngestError::Page { path, source: _ } => {
            assert!(
                path.ends_with("missing-summary.md"),
                "path={path}: should point at the offending file"
            );
        }
        other => panic!("expected IngestError::Page, got {other:?}"),
    }
}

#[test]
fn ingestor_root_accessor_returns_what_was_passed() {
    let root = fixture("pages");
    let ingestor = MarkdownPagesIngestor::new(root.clone());
    assert_eq!(ingestor.root(), root);
}
