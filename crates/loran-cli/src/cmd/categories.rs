// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran categories` — registry of categories with live counts.

use std::io::Write as _;
use std::process::ExitCode;

use loran_core::{BundledPagesIngestor, bundled_categories};
use loran_index::{Index, Ingestor};
use serde::Serialize;

use crate::cli::{CategoriesArgs, Cli, Format};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

/// One row in the `loran categories` data array.
#[derive(Serialize)]
struct CategoryRow<'a> {
    /// Slug (e.g. `file-listing`).
    name: &'a str,
    /// Human-readable title from `categories.toml`.
    title: &'a str,
    /// One-sentence description.
    description: &'a str,
    /// Live page count, drawn from `Index::category_count`.
    count: usize,
}

pub(crate) fn run(cli: &Cli, _args: &CategoriesArgs) -> ExitCode {
    let index = match build_index() {
        Ok(idx) => idx,
        Err(msg) => {
            emit_index_failure(cli, &msg);
            return ExitCode::from(LoranExit::IndexNotBuilt.to_process_code());
        }
    };

    let cats = match bundled_categories() {
        Ok(c) => c,
        Err(err) => {
            emit_registry_failure(cli, &err.to_string());
            return ExitCode::from(LoranExit::PageParseError.to_process_code());
        }
    };

    let rows: Vec<CategoryRow<'_>> = cats
        .iter()
        .map(|(slug, entry)| CategoryRow {
            name: slug,
            title: entry.title.as_str(),
            description: entry.description.as_str(),
            count: index.category_count(slug),
        })
        .collect();

    match cli.output_format() {
        Format::Json => emit_json(&rows),
        Format::Human => emit_text(&rows),
    }

    ExitCode::from(0)
}

fn build_index() -> Result<Index, String> {
    let pages = BundledPagesIngestor::new()
        .ingest()
        .map_err(|e| format!("bundled-pages ingest failed: {e}"))?;
    Index::build(pages).map_err(|e| format!("index build failed: {e}"))
}

fn emit_text(rows: &[CategoryRow<'_>]) {
    let mut stdout = std::io::stdout().lock();
    for row in rows {
        let _ = writeln!(stdout, "{}\t{}\t{}", row.name, row.title, row.count);
    }
}

fn emit_json(rows: &[CategoryRow<'_>]) {
    let envelope = Envelope::new("loran categories", rows);
    let _ = JsonEmitter::stdio().emit_data(&envelope);
}

fn emit_index_failure(cli: &Cli, msg: &str) {
    let ctx = ErrorContext::empty();
    let hint = LoranExit::IndexNotBuilt.hint(&ctx);
    emit_error(cli, LoranExit::IndexNotBuilt, msg, &hint);
}

fn emit_registry_failure(cli: &Cli, msg: &str) {
    let ctx = ErrorContext::empty();
    let hint = LoranExit::PageParseError.hint(&ctx);
    emit_error(cli, LoranExit::PageParseError, msg, &hint);
}

fn emit_error(cli: &Cli, code: LoranExit, msg: &str, hint: &str) {
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                msg,
                hint,
                "loran categories",
                None,
            );
            let _ = JsonEmitter::stdio().emit_error(&envelope);
        }
        Format::Human => {
            eprintln!("error: {msg}");
            eprintln!("  hint: {hint}");
        }
    }
}
