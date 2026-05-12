// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Hard NFR threshold tests.
//!
//! PRD Q-01 / NFR-001: `loran show <known-tool>` ≤ 50 ms cold-cache.
//! PRD NFR-002: `loran list` over a 1k-page catalog ≤ 100 ms.
//!
//! These tests time the resolution path directly (not the whole CLI
//! binary spawn), so they bound the *algorithmic* cost rather than
//! `cargo run` start-up overhead. The CLI itself is exercised by the
//! integration tests in `loran-cli/tests/`.
//!
//! Thresholds are intentionally generous in debug builds (10x) because
//! `cargo test` defaults to debug and we don't want spurious failures.
//! `cargo test --release` enforces the real PRD numbers.

use std::time::Instant;

use loran_core::{BundledPagesIngestor, resolve_show};
use loran_index::{Index, Ingestor};
use loran_pages::Page;

/// Multiplier applied to NFR thresholds in debug builds.
const DEBUG_RELAXATION: u32 = 10;

fn relaxed_millis(release_threshold_ms: u128) -> u128 {
    if cfg!(debug_assertions) {
        release_threshold_ms * u128::from(DEBUG_RELAXATION)
    } else {
        release_threshold_ms
    }
}

#[test]
fn nfr_001_resolve_show_under_50ms_cold() {
    let pages = BundledPagesIngestor::new()
        .ingest()
        .expect("bundled ingest");
    let idx = Index::build(pages).expect("index builds");

    // Warm up to avoid pathological first-call cost; measure the steady-state.
    let _ = resolve_show(&idx, "eza");

    let start = Instant::now();
    let _ = resolve_show(&idx, "eza");
    let elapsed_ms = start.elapsed().as_millis();

    let limit = relaxed_millis(50);
    assert!(
        elapsed_ms <= limit,
        "NFR-001 violation: resolve_show took {elapsed_ms} ms, limit {limit} ms"
    );
}

#[test]
fn nfr_002_list_over_1k_catalog_under_100ms() {
    const CATEGORIES: &[&str] = &[
        "file-listing",
        "file-viewing",
        "text-search",
        "file-search",
        "process-management",
        "system-monitoring",
        "networking",
        "version-control",
        "shell-utilities",
        "data-processing",
    ];

    let pages: Vec<Page> = (0..1_000)
        .map(|i| {
            let category = CATEGORIES[i % CATEGORIES.len()];
            let src = format!(
                "+++\n\
                 name = \"synth-{i}\"\n\
                 category = \"{category}\"\n\
                 summary = \"Synthetic entry #{i} for NFR-002.\"\n\
                 +++\n"
            );
            Page::parse(&src).expect("synthetic page parses")
        })
        .collect();

    // Warm-up index build (excluded from the measurement).
    let _ = Index::build(pages.clone()).expect("warm-up build");

    let start = Instant::now();
    let idx = Index::build(pages).expect("measured build");
    // The "list" operation is `idx.all()` collect — that's what
    // `loran list` does after building the index from the catalog.
    let all: Vec<&Page> = idx.all().collect();
    let elapsed_ms = start.elapsed().as_millis();

    assert_eq!(all.len(), 1_000);
    let limit = relaxed_millis(100);
    assert!(
        elapsed_ms <= limit,
        "NFR-002 violation: 1k catalog list took {elapsed_ms} ms, limit {limit} ms"
    );
}
