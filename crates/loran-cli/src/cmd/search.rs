// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran search <query>` — fuzzy match across the catalog.

use std::io::Write as _;
use std::process::ExitCode;

use loran_core::{BundledPagesIngestor, SearchResult, resolve_search};
use loran_index::{Index, Ingestor};

use crate::cli::{Cli, Format, SearchArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

pub(crate) fn run(cli: &Cli, args: &SearchArgs) -> ExitCode {
    let index = match build_index() {
        Ok(idx) => idx,
        Err(msg) => {
            emit_index_failure(cli, &msg, &args.query);
            return ExitCode::from(LoranExit::IndexNotBuilt.to_process_code());
        }
    };

    let result = resolve_search(&index, &args.query);

    match cli.output_format() {
        Format::Json => emit_json(&result),
        Format::Human => emit_text(&result),
    }

    ExitCode::from(0)
}

fn build_index() -> Result<Index, String> {
    let pages = BundledPagesIngestor::new()
        .ingest()
        .map_err(|e| format!("bundled-pages ingest failed: {e}"))?;
    Index::build(pages).map_err(|e| format!("index build failed: {e}"))
}

fn emit_text(result: &SearchResult) {
    let mut stdout = std::io::stdout().lock();
    if result.matches.is_empty() {
        let _ = writeln!(stdout, "no matches for `{}`", result.query);
        let _ = writeln!(
            stdout,
            "  hint: loran list --json   # browse the full catalog"
        );
        return;
    }
    for m in &result.matches {
        let _ = writeln!(
            stdout,
            "{}\t{}\t{}\t{}",
            m.score, m.page.name, m.page.category, m.page.summary
        );
    }
}

fn emit_json(result: &SearchResult) {
    let envelope = Envelope::new(format!("loran search {}", result.query), result);
    let _ = JsonEmitter::stdio().emit_data(&envelope);
}

fn emit_index_failure(cli: &Cli, msg: &str, query: &str) {
    let ctx = ErrorContext::with_query(query);
    let hint = LoranExit::IndexNotBuilt.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                LoranExit::IndexNotBuilt.name(),
                LoranExit::IndexNotBuilt.numeric(),
                msg,
                &hint,
                format!("loran search {query}"),
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
