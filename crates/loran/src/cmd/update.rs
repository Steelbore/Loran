// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran update` — refresh both the upstream Spacecraft Software pages tarball
//! and the tldr-pages mirror in a single invocation.
//!
//! Composes the Sub-phase 2A/2B primitives via
//! [`loran_core::update_pages`] and [`loran_core::update_tldr`]. Each
//! source runs independently; the envelope's `data.results: [...]`
//! array surfaces per-source status. The process exit code is the
//! worst (highest-numbered) code across all sources — a single source
//! failure does not block the other from running.

use std::process::ExitCode;

use loran_core::{
    FetchClient, SourceMetaStore, UpdateError, UpdateOpts, UpdateOutcome, UpdateTldrOpts,
    default_pages_target, default_tldr_target, update_pages, update_tldr,
};
use serde::Serialize;

use crate::cli::{Cli, Format, UpdateArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

/// Per-source row in the `loran update` JSON envelope.
#[derive(Serialize)]
struct SourceResult {
    source: &'static str,
    status: &'static str,
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<SourceErrorBlock>,
}

#[derive(Serialize)]
struct SourceErrorBlock {
    code: &'static str,
    exit_code: i32,
    message: String,
    hint: String,
}

#[derive(Serialize)]
struct UpdateData {
    results: Vec<SourceResult>,
}

pub(crate) fn run(cli: &Cli, args: &UpdateArgs) -> ExitCode {
    let meta_store = match SourceMetaStore::with_default_path() {
        Ok(s) => s,
        Err(err) => {
            emit_meta_failure(cli, &err.to_string());
            return ExitCode::from(LoranExit::PermissionDenied.to_process_code());
        }
    };

    let client = FetchClient::new();
    let mut results: Vec<SourceResult> = Vec::with_capacity(2);
    let mut worst_exit: u8 = 0;

    // --- Source 1: upstream-pages ---
    if let Some(target) = default_pages_target() {
        let mut opts = UpdateOpts::default_publisher(target);
        opts.dry_run = cli.global.dry_run;
        opts.force_refresh = args.force_refresh;
        let (row, exit) = run_pages(&client, &meta_store, &opts);
        worst_exit = worst_exit.max(exit);
        results.push(row);
    } else {
        worst_exit = worst_exit.max(LoranExit::PermissionDenied.to_process_code());
        results.push(missing_target_row(
            "upstream-pages",
            "no $XDG_DATA_HOME / data directory available on this platform",
        ));
    }

    // --- Source 2: tldr-pages ---
    if let Some(target) = default_tldr_target() {
        let mut opts = UpdateTldrOpts::default_publisher(target);
        opts.dry_run = cli.global.dry_run;
        opts.force_refresh = args.force_refresh;
        let (row, exit) = run_tldr(&client, &meta_store, &opts);
        worst_exit = worst_exit.max(exit);
        results.push(row);
    } else {
        worst_exit = worst_exit.max(LoranExit::PermissionDenied.to_process_code());
        results.push(missing_target_row(
            "tldr-pages",
            "no $XDG_CACHE_HOME / cache directory available on this platform",
        ));
    }

    emit_envelope(cli, &results);
    ExitCode::from(worst_exit)
}

fn run_pages(
    client: &FetchClient,
    store: &SourceMetaStore,
    opts: &UpdateOpts,
) -> (SourceResult, u8) {
    match update_pages(client, store, opts) {
        Ok(outcome) => (
            success_row("upstream-pages", opts.dry_run, &outcome),
            LoranExit::Success.to_process_code(),
        ),
        Err(err) => {
            let code = classify(&err);
            (
                error_row("upstream-pages", opts.dry_run, code, &err),
                code.to_process_code(),
            )
        }
    }
}

fn run_tldr(
    client: &FetchClient,
    store: &SourceMetaStore,
    opts: &UpdateTldrOpts,
) -> (SourceResult, u8) {
    match update_tldr(client, store, opts) {
        Ok(outcome) => (
            success_row("tldr-pages", opts.dry_run, &outcome),
            LoranExit::Success.to_process_code(),
        ),
        Err(err) => {
            let code = classify(&err);
            (
                error_row("tldr-pages", opts.dry_run, code, &err),
                code.to_process_code(),
            )
        }
    }
}

fn classify(err: &UpdateError) -> LoranExit {
    match err {
        UpdateError::Sign(_) | UpdateError::Extract(_) => LoranExit::TarballVerifyFailed,
        UpdateError::Fetch(_) | UpdateError::UnexpectedNotModified => LoranExit::TarballFetchFailed,
        UpdateError::Meta(_) => LoranExit::PermissionDenied,
    }
}

fn success_row(source: &'static str, dry_run: bool, outcome: &UpdateOutcome) -> SourceResult {
    let (label, version, etag) = match outcome {
        UpdateOutcome::NotModified => ("not_modified", None, None),
        UpdateOutcome::Updated { version, etag } => {
            ("updated", Some(version.clone()), Some(etag.clone()))
        }
        UpdateOutcome::DryRun { version, etag } => {
            ("dry_run", Some(version.clone()), Some(etag.clone()))
        }
    };
    SourceResult {
        source,
        status: "ok",
        outcome: label,
        version,
        etag,
        dry_run,
        error: None,
    }
}

fn error_row(
    source: &'static str,
    dry_run: bool,
    code: LoranExit,
    err: &UpdateError,
) -> SourceResult {
    let ctx = ErrorContext::empty();
    SourceResult {
        source,
        status: "error",
        outcome: "failed",
        version: None,
        etag: None,
        dry_run,
        error: Some(SourceErrorBlock {
            code: code.name(),
            exit_code: code.numeric(),
            message: err.to_string(),
            hint: code.hint(&ctx),
        }),
    }
}

fn missing_target_row(source: &'static str, message: &str) -> SourceResult {
    let code = LoranExit::PermissionDenied;
    let ctx = ErrorContext::empty();
    SourceResult {
        source,
        status: "error",
        outcome: "failed",
        version: None,
        etag: None,
        dry_run: false,
        error: Some(SourceErrorBlock {
            code: code.name(),
            exit_code: code.numeric(),
            message: message.to_owned(),
            hint: code.hint(&ctx),
        }),
    }
}

fn emit_envelope(cli: &Cli, results: &[SourceResult]) {
    match cli.output_format() {
        Format::Json => {
            let payload = UpdateData {
                results: results
                    .iter()
                    .map(|r| SourceResult {
                        source: r.source,
                        status: r.status,
                        outcome: r.outcome,
                        version: r.version.clone(),
                        etag: r.etag.clone(),
                        dry_run: r.dry_run,
                        error: r.error.as_ref().map(|e| SourceErrorBlock {
                            code: e.code,
                            exit_code: e.exit_code,
                            message: e.message.clone(),
                            hint: e.hint.clone(),
                        }),
                    })
                    .collect(),
            };
            let envelope = Envelope::new("loran update", payload);
            let _ = JsonEmitter::stdio().emit_data(&envelope);
        }
        Format::Human => {
            for row in results {
                emit_row_human(row);
            }
        }
    }
}

fn emit_row_human(row: &SourceResult) {
    if let Some(err) = row.error.as_ref() {
        eprintln!("{}: error: {}", row.source, err.message);
        eprintln!("  hint: {}", err.hint);
        return;
    }
    match row.outcome {
        "not_modified" => println!("{}: already up to date", row.source),
        "updated" => {
            let v = row.version.as_deref().unwrap_or("(unknown version)");
            println!("{}: updated to {}", row.source, v);
        }
        "dry_run" => {
            let v = row.version.as_deref().unwrap_or("(unknown version)");
            println!("{}: dry-run — would update to {}", row.source, v);
        }
        other => println!("{}: {}", row.source, other),
    }
}

fn emit_meta_failure(cli: &Cli, msg: &str) {
    let code = LoranExit::PermissionDenied;
    let ctx = ErrorContext::empty();
    let hint = code.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                msg,
                &hint,
                "loran update",
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
