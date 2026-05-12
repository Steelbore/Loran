// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran search <query>` — fuzzy match across the catalog.

use std::io::Write as _;
use std::process::ExitCode;

use loran_core::{SearchResult, resolve_search};
use serde::Serialize;

use crate::cli::{Cli, Format, SearchArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};
use crate::index_loader::build_layered_index;
use crate::summary::PageSummary;

/// JSON projection of [`SearchResult`] — replaces each match's `Page`
/// with a `PageSummary` so list-shaped output stays metadata-only.
#[derive(Serialize)]
struct SearchData<'a> {
    query: &'a str,
    matches: Vec<ScoredSummary<'a>>,
}

#[derive(Serialize)]
struct ScoredSummary<'a> {
    page: PageSummary<'a>,
    score: u32,
}

impl<'a> From<&'a SearchResult> for SearchData<'a> {
    fn from(result: &'a SearchResult) -> Self {
        Self {
            query: result.query.as_str(),
            matches: result
                .matches
                .iter()
                .map(|m| ScoredSummary {
                    page: PageSummary::from(&m.page),
                    score: m.score,
                })
                .collect(),
        }
    }
}

pub(crate) fn run(cli: &Cli, args: &SearchArgs) -> ExitCode {
    let index = match build_layered_index() {
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
    let data = SearchData::from(result);
    let envelope = Envelope::new(format!("loran search {}", result.query), data);
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
