// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran update` — refresh the upstream Steelbore pages tarball.
//!
//! Composes the Sub-phase 2A primitives via `loran_core::update_pages`.
//! The Phase 1 stub message is replaced with a real fetch → verify →
//! extract → meta-store pipeline.
//!
//! Phase 2B scope: upstream pages only. The tldr-pages mirror lands
//! in `WP-P2.11`.

use std::process::ExitCode;

use loran_core::{
    FetchClient, SourceMetaStore, UpdateError, UpdateOpts, UpdateOutcome, default_pages_target,
    update_pages,
};
use serde::Serialize;

use crate::cli::{Cli, Format, UpdateArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

#[derive(Serialize)]
struct UpdateData {
    source: &'static str,
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    dry_run: bool,
}

pub(crate) fn run(cli: &Cli, args: &UpdateArgs) -> ExitCode {
    let Some(target) = default_pages_target() else {
        emit_failure(
            cli,
            LoranExit::PermissionDenied,
            "no $XDG_DATA_HOME / cache directory available on this platform",
        );
        return ExitCode::from(LoranExit::PermissionDenied.to_process_code());
    };

    let mut opts = UpdateOpts::default_publisher(target);
    opts.dry_run = cli.global.dry_run;
    opts.force_refresh = args.force_refresh;

    let meta_store = match SourceMetaStore::with_default_path() {
        Ok(s) => s,
        Err(err) => {
            emit_failure(cli, LoranExit::PermissionDenied, &err.to_string());
            return ExitCode::from(LoranExit::PermissionDenied.to_process_code());
        }
    };

    let client = FetchClient::new();

    match update_pages(&client, &meta_store, &opts) {
        Ok(outcome) => emit_success(cli, &opts.source_name, opts.dry_run, &outcome),
        Err(err) => emit_pipeline_failure(cli, &err),
    }
}

fn emit_success(cli: &Cli, source: &str, dry_run: bool, outcome: &UpdateOutcome) -> ExitCode {
    let (outcome_label, version, etag) = match outcome {
        UpdateOutcome::NotModified => ("not_modified", None, None),
        UpdateOutcome::Updated { version, etag } => {
            ("updated", Some(version.clone()), Some(etag.clone()))
        }
        UpdateOutcome::DryRun { version, etag } => {
            ("dry_run", Some(version.clone()), Some(etag.clone()))
        }
    };

    match cli.output_format() {
        Format::Json => {
            // Source name needs to be 'static; intern via Box::leak for
            // the small fixed set we know about.
            let interned_source = intern_source(source);
            let envelope = Envelope::new(
                format!("loran update ({source})"),
                UpdateData {
                    source: interned_source,
                    outcome: outcome_label,
                    version,
                    etag,
                    dry_run,
                },
            );
            let _ = JsonEmitter::stdio().emit_data(&envelope);
        }
        Format::Human => match outcome {
            UpdateOutcome::NotModified => {
                println!("{source}: already up to date");
            }
            UpdateOutcome::Updated { version, .. } => {
                println!("{source}: updated to {version}");
            }
            UpdateOutcome::DryRun { version, .. } => {
                println!("{source}: dry-run — would update to {version}");
            }
        },
    }

    ExitCode::from(0)
}

fn emit_pipeline_failure(cli: &Cli, err: &UpdateError) -> ExitCode {
    let code = match err {
        UpdateError::Sign(_) | UpdateError::Extract(_) => LoranExit::TarballVerifyFailed,
        UpdateError::Fetch(_) | UpdateError::UnexpectedNotModified => LoranExit::TarballFetchFailed,
        UpdateError::Meta(_) => LoranExit::PermissionDenied,
    };
    let msg = err.to_string();
    let ctx = ErrorContext::empty();
    let hint = code.hint(&ctx);

    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                &msg,
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

    ExitCode::from(code.to_process_code())
}

fn emit_failure(cli: &Cli, code: LoranExit, msg: &str) {
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

/// Convert a runtime source name to a `&'static str`. We know the small
/// closed set (`upstream-pages`, `tldr-pages`, etc.), so a `match` over
/// the known names is cleaner than `Box::leak`-ing arbitrary strings.
fn intern_source(name: &str) -> &'static str {
    match name {
        "upstream-pages" => "upstream-pages",
        "tldr-pages" => "tldr-pages",
        _ => "unknown",
    }
}
