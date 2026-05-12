// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Criterion benchmarks for the read-side resolution chain.
//!
//! Covers the three Phase 1 entry points (`resolve_show`,
//! `resolve_find`, `resolve_search`) over the bundled seed catalog
//! (PRD M-01 reference data set) and a synthetic 1k-page catalog
//! (PRD NFR-002 reference load).
//!
//! Run with `cargo bench` — the harness is set to `false` in
//! `Cargo.toml` so criterion's own main is used.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use loran_core::{BundledPagesIngestor, resolve_find, resolve_search, resolve_show};
use loran_index::{Index, Ingestor};
use loran_pages::Page;

fn bundled_index() -> Index {
    let pages = BundledPagesIngestor::new()
        .ingest()
        .expect("bundled pages ingest");
    Index::build(pages).expect("bundled index builds")
}

/// Synthesise a 1k-page in-memory catalog covering 10 categories.
fn synthetic_index_1k() -> Index {
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
                 summary = \"Synthetic entry #{i} for NFR-002 benchmarking.\"\n\
                 replaces = [\"legacy-{i}\"]\n\
                 tags = [\"synthetic\", \"bench\"]\n\
                 +++\n\
                 Body of synthetic entry {i}.\n"
            );
            Page::parse(&src).expect("synthetic page parses")
        })
        .collect();
    Index::build(pages).expect("synthetic index builds")
}

fn bench_show(c: &mut Criterion) {
    let idx = bundled_index();
    c.bench_function("resolve_show/bundled/hit", |b| {
        b.iter(|| {
            let r = resolve_show(black_box(&idx), black_box("eza"));
            black_box(r);
        });
    });
    c.bench_function("resolve_show/bundled/miss", |b| {
        b.iter(|| {
            let r = resolve_show(black_box(&idx), black_box("no-such-tool"));
            black_box(r);
        });
    });
}

fn bench_find(c: &mut Criterion) {
    let idx = bundled_index();
    c.bench_function("resolve_find/bundled/broad", |b| {
        b.iter(|| {
            let r = resolve_find(black_box(&idx), black_box("cat"), black_box(false));
            black_box(r);
        });
    });
    c.bench_function("resolve_find/bundled/safe-alias", |b| {
        b.iter(|| {
            let r = resolve_find(black_box(&idx), black_box("cat"), black_box(true));
            black_box(r);
        });
    });
}

fn bench_search(c: &mut Criterion) {
    let bundled = bundled_index();
    let synthetic = synthetic_index_1k();

    c.bench_function("resolve_search/bundled/single-word", |b| {
        b.iter(|| {
            let r = resolve_search(black_box(&bundled), black_box("modern"));
            black_box(r);
        });
    });
    c.bench_function("resolve_search/synthetic-1k/single-word", |b| {
        b.iter(|| {
            let r = resolve_search(black_box(&synthetic), black_box("synth-500"));
            black_box(r);
        });
    });
    c.bench_function("resolve_search/synthetic-1k/tag-match", |b| {
        b.iter(|| {
            let r = resolve_search(black_box(&synthetic), black_box("synthetic"));
            black_box(r);
        });
    });
}

fn bench_index_build(c: &mut Criterion) {
    let pages = BundledPagesIngestor::new()
        .ingest()
        .expect("bundled pages ingest");
    c.bench_function("Index::build/bundled", |b| {
        b.iter(|| {
            let idx = Index::build(black_box(pages.clone())).expect("build");
            black_box(idx);
        });
    });
}

criterion_group!(
    benches,
    bench_show,
    bench_find,
    bench_search,
    bench_index_build
);
criterion_main!(benches);
