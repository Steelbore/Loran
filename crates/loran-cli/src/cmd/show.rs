// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran show <tool>` — render the curated page for a tool, or fail
//! with `NOT_FOUND` if the catalog has no entry (Spec §4.1 — never
//! falls through to live `--help`; that's the separate `help` verb).

use std::io::Write as _;
use std::process::ExitCode;

use loran_core::{BodyBlock, BundledPagesIngestor, IntroBlock, ShowResult, resolve_show};
use loran_index::{Index, Ingestor};
use loran_pages::Page;
use loran_render::render_text;
use serde::Serialize;

use crate::cli::{Cli, Format, ShowArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

/// `loran show` data payload — Spec §8 envelope shape.
#[derive(Serialize)]
struct ShowData {
    #[serde(flatten)]
    page: Page,
    intro: IntroBlock,
    body: BodyBlock,
}

pub(crate) fn run(cli: &Cli, args: &ShowArgs) -> ExitCode {
    let index = match build_index() {
        Ok(idx) => idx,
        Err(msg) => {
            emit_index_failure(cli, &msg, &args.tool);
            return ExitCode::from(LoranExit::IndexNotBuilt.to_process_code());
        }
    };

    match resolve_show(&index, &args.tool) {
        ShowResult::IndexHit { page, intro, body } => emit_hit(cli, &page, intro, body),
        ShowResult::NoEntry { tool, hint } => emit_no_entry(cli, &tool, &hint),
    }
}

fn build_index() -> Result<Index, String> {
    let pages = BundledPagesIngestor::new()
        .ingest()
        .map_err(|e| format!("bundled-pages ingest failed: {e}"))?;
    Index::build(pages).map_err(|e| format!("index build failed: {e}"))
}

fn emit_hit(cli: &Cli, page: &Page, intro: IntroBlock, body: BodyBlock) -> ExitCode {
    match cli.output_format() {
        Format::Json => emit_hit_json(page, intro, body),
        Format::Human => emit_hit_human(page, &intro, &body),
    }
    ExitCode::from(0)
}

fn emit_hit_human(page: &Page, intro: &IntroBlock, body: &BodyBlock) {
    let mut stdout = std::io::stdout().lock();

    // Banner: TOOL NAME + underline of '=' the width of the name.
    let _ = writeln!(stdout, "{}", page.name.to_uppercase());
    let _ = writeln!(stdout, "{}", "=".repeat(page.name.chars().count()));
    let _ = writeln!(stdout);
    let _ = writeln!(stdout, "{}", intro.body_md);
    let _ = writeln!(stdout);
    let _ = writeln!(stdout, "---");
    let _ = writeln!(stdout);

    // Render the body via the loran-render text path. The writer is
    // stdout — any failure is an I/O error against the terminal and
    // there is no useful recovery; swallow.
    if render_text(&body.body_md, &mut stdout).is_err() {
        let _ = stdout.write_all(body.body_md.as_bytes());
    }
}

fn emit_hit_json(page: &Page, intro: IntroBlock, body: BodyBlock) {
    let command = format!("loran show {}", page.name);
    let envelope = Envelope::new(
        command,
        ShowData {
            page: page.clone(),
            intro,
            body,
        },
    );
    let _ = JsonEmitter::stdio().emit_data(&envelope);
}

fn emit_no_entry(cli: &Cli, tool: &str, hint: &str) -> ExitCode {
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                LoranExit::NotFound.name(),
                LoranExit::NotFound.numeric(),
                format!("no Loran entry for '{tool}'"),
                hint,
                format!("loran show {tool}"),
                None,
            );
            let _ = JsonEmitter::stdio().emit_error(&envelope);
        }
        Format::Human => {
            eprintln!("error: no Loran entry for '{tool}'");
            eprintln!();
            eprintln!("  hint: {hint}");
            eprintln!("        (scaffolds a page in your user overlay; opens $EDITOR)");
            eprintln!();
            eprintln!("  see also: loran search {tool} --json");
            eprintln!("            loran help {tool}  (capture upstream --help directly)");
        }
    }
    ExitCode::from(LoranExit::NotFound.to_process_code())
}

fn emit_index_failure(cli: &Cli, msg: &str, tool: &str) {
    let ctx = ErrorContext::with_tool(tool);
    let hint = LoranExit::IndexNotBuilt.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                LoranExit::IndexNotBuilt.name(),
                LoranExit::IndexNotBuilt.numeric(),
                msg,
                &hint,
                format!("loran show {tool}"),
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
