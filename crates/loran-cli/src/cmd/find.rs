// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran find <legacy>` — reverse legacy-name lookup.

use std::io::Write as _;
use std::process::ExitCode;

use loran_core::{BundledPagesIngestor, FindResult, resolve_find};
use loran_index::{Index, Ingestor};
use serde::Serialize;

use crate::cli::{Cli, FindArgs, Format};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};
use crate::summary::PageSummary;

/// JSON projection of [`FindResult`] — replaces `Vec<Page>` with a
/// `Vec<PageSummary>` so list-shaped output stays metadata-only.
#[derive(Serialize)]
struct FindData<'a> {
    query: &'a str,
    safe_alias_only: bool,
    matches: Vec<PageSummary<'a>>,
}

impl<'a> From<&'a FindResult> for FindData<'a> {
    fn from(result: &'a FindResult) -> Self {
        Self {
            query: result.query.as_str(),
            safe_alias_only: result.safe_alias_only,
            matches: result.matches.iter().map(PageSummary::from).collect(),
        }
    }
}

pub(crate) fn run(cli: &Cli, args: &FindArgs) -> ExitCode {
    let index = match build_index() {
        Ok(idx) => idx,
        Err(msg) => {
            emit_index_failure(cli, &msg, &args.legacy);
            return ExitCode::from(LoranExit::IndexNotBuilt.to_process_code());
        }
    };

    let result = resolve_find(&index, &args.legacy, args.safe_alias);

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

fn emit_text(result: &FindResult) {
    let mut stdout = std::io::stdout().lock();
    if result.matches.is_empty() {
        let qualifier = if result.safe_alias_only {
            " alias-safely"
        } else {
            ""
        };
        let _ = writeln!(
            stdout,
            "no Steelbore tool{} supersedes `{}`",
            qualifier, result.query
        );
        let _ = writeln!(stdout, "  hint: loran search {} --json", result.query);
        return;
    }
    for page in &result.matches {
        let _ = writeln!(stdout, "{}\t{}\t{}", page.name, page.category, page.summary);
    }
}

fn emit_json(result: &FindResult) {
    let data = FindData::from(result);
    let envelope = Envelope::new(format!("loran find {}", result.query), data);
    let _ = JsonEmitter::stdio().emit_data(&envelope);
}

fn emit_index_failure(cli: &Cli, msg: &str, legacy: &str) {
    let ctx = ErrorContext::with_query(legacy);
    let hint = LoranExit::IndexNotBuilt.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                LoranExit::IndexNotBuilt.name(),
                LoranExit::IndexNotBuilt.numeric(),
                msg,
                &hint,
                format!("loran find {legacy}"),
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
