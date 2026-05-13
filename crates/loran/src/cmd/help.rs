// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran help <tool>` — live `--help` capture with a deliberately
//! de-themed monochrome frame (Spec §4.2 step 3, decision #11).

use std::io::Write as _;
use std::process::ExitCode;

use loran_core::{HelpError, HelpOpts, HelpResult, PagerSource, capture_help};
use serde::Serialize;

use crate::cli::{Cli, Format, HelpArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

/// `--pager=loran` sentinel intercepted at the CLI layer.
const LORAN_PAGER_SENTINEL: &str = "loran";

#[derive(Serialize)]
struct HelpData {
    tool: String,
    body: HelpBody,
}

#[derive(Serialize)]
struct HelpBody {
    /// Always `"live_help"` for this verb (Spec §4.2).
    kind: &'static str,
    /// Captured stdout (or stderr if stdout was empty).
    captured_text: String,
    /// Spawn timestamp (ISO 8601 UTC, Z suffix).
    captured_at: String,
    /// Which flag form produced the output (`--help`, `-h`, `help`).
    flag_used: &'static str,
    /// Exit code of the captured subprocess.
    exit_code: i32,
    /// Resolved pager command per §4.2.1 cascade.
    pager_command: String,
    /// Which cascade step won (Spec §4.2.1 JSON envelope contract).
    pager_source: PagerSource,
}

pub(crate) fn run(cli: &Cli, args: &HelpArgs) -> ExitCode {
    let opts = build_opts(args);

    match capture_help(&args.tool, &opts) {
        Ok(result) => emit_success(cli, &args.tool, &result),
        Err(err) => emit_failure(cli, &args.tool, &err),
    }
}

/// Convert CLI flag input to `HelpOpts`, intercepting the
/// `--pager=loran` sentinel.
fn build_opts(args: &HelpArgs) -> HelpOpts {
    match args.pager.as_deref() {
        Some(LORAN_PAGER_SENTINEL) => HelpOpts {
            pager: None,
            skip_user_env_pager: true,
            timeout: None,
        },
        Some(other) => HelpOpts {
            pager: Some(other.to_owned()),
            skip_user_env_pager: false,
            timeout: None,
        },
        None => HelpOpts::default(),
    }
}

fn emit_success(cli: &Cli, tool: &str, result: &HelpResult) -> ExitCode {
    match cli.output_format() {
        Format::Json => emit_json(tool, result),
        Format::Human => emit_text(tool, result),
    }
    ExitCode::from(0)
}

fn emit_text(tool: &str, result: &HelpResult) {
    let mut stdout = std::io::stdout().lock();
    // Monochrome ASCII frame — deliberately NOT the Steelbore palette
    // (Spec §2 decision #11; §4.2 step 3). The brand boundary lives
    // here.
    let header = format!(
        "LIVE OUTPUT — uncurated, captured from `{tool} {}` at {}",
        result.flag_used_token(),
        result.captured_at
    );
    let rule = "-".repeat(header.chars().count().min(80));
    let _ = writeln!(stdout, "{header}");
    let _ = writeln!(stdout, "{rule}");
    let _ = stdout.write_all(result.captured_text.as_bytes());
    if !result.captured_text.ends_with('\n') {
        let _ = writeln!(stdout);
    }
    let _ = writeln!(stdout, "{rule}");
    let _ = writeln!(
        stdout,
        "[pager: {} ({}); exit: {}]",
        result.pager_command,
        pager_source_label(result.pager_source),
        result.exit_code
    );
}

fn emit_json(tool: &str, result: &HelpResult) {
    let data = HelpData {
        tool: tool.to_owned(),
        body: HelpBody {
            kind: "live_help",
            captured_text: result.captured_text.clone(),
            captured_at: result.captured_at.to_string(),
            flag_used: result.flag_used_token(),
            exit_code: result.exit_code,
            pager_command: result.pager_command.clone(),
            pager_source: result.pager_source,
        },
    };
    let envelope = Envelope::new(format!("loran help {tool}"), data);
    let _ = JsonEmitter::stdio().emit_data(&envelope);
}

fn emit_failure(cli: &Cli, tool: &str, err: &HelpError) -> ExitCode {
    let (code, msg) = match err {
        HelpError::Timeout { .. } => (LoranExit::LiveHelpTimeout, err.to_string()),
        HelpError::BinaryNotFound(_) | HelpError::PathLikeName(_) => {
            (LoranExit::NotFound, err.to_string())
        }
        HelpError::SpawnFailed { .. } | HelpError::AllFlagsFailed(_) => {
            (LoranExit::GeneralError, err.to_string())
        }
    };
    let ctx = ErrorContext::with_tool(tool);
    let hint = code.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                &msg,
                &hint,
                format!("loran help {tool}"),
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

fn pager_source_label(source: PagerSource) -> &'static str {
    match source {
        PagerSource::Flag => "flag",
        PagerSource::ManpagerEnv => "manpager-env",
        PagerSource::PagerEnv => "pager-env",
        PagerSource::Bat => "bat",
        PagerSource::Moor => "moor",
        PagerSource::Cat => "cat",
    }
}

/// Helper extension: token for the flag-used field.
trait FlagUsedToken {
    fn flag_used_token(&self) -> &'static str;
}

impl FlagUsedToken for HelpResult {
    fn flag_used_token(&self) -> &'static str {
        match self.flag_used {
            loran_core::HelpFlag::Help => "--help",
            loran_core::HelpFlag::H => "-h",
            loran_core::HelpFlag::HelpSub => "help",
        }
    }
}
