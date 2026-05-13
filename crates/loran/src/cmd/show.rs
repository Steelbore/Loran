// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran show <tool>` — render the curated page for a tool, or fail
//! with `NOT_FOUND` if the catalog has no entry (Spec §4.1 — never
//! falls through to live `--help`; that's the separate `help` verb).

use std::io::Write as _;
use std::process::ExitCode;

use loran_core::{
    BodyBlock, IntroBlock, NoTldr, ShowResult, TldrCache, TldrLookup, resolve_show_with_tldr,
};
use loran_pages::Page;
use loran_render::render_text;
use serde::Serialize;

use crate::cli::{Cli, Format, ShowArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};
use crate::index_loader::build_layered_index_with_overlay;

/// `loran show` data payload — Spec §8 envelope shape.
///
/// Projects [`Page`] metadata explicitly (rather than via
/// `#[serde(flatten)]`) so the nested `body: BodyBlock` field never
/// collides with the raw `Page::body` markdown string.
#[derive(Serialize)]
struct ShowData<'a> {
    name: &'a str,
    category: &'a str,
    summary: &'a str,
    replaces: &'a [String],
    safe_alias_for: &'a [String],
    pairs_with: &'a [String],
    official: Option<&'a str>,
    tldr_page: Option<&'a str>,
    tags: &'a [String],
    written_in: Option<&'a str>,
    language: Option<&'a str>,
    since: Option<&'a str>,
    aliases: &'a [String],
    intro: IntroBlock,
    body: BodyBlock,
}

impl<'a> ShowData<'a> {
    fn from_parts(page: &'a Page, intro: IntroBlock, body: BodyBlock) -> Self {
        Self {
            name: &page.name,
            category: &page.category,
            summary: &page.summary,
            replaces: &page.replaces,
            safe_alias_for: &page.safe_alias_for,
            pairs_with: &page.pairs_with,
            official: page.official.as_deref(),
            tldr_page: page.tldr_page.as_deref(),
            tags: &page.tags,
            written_in: page.written_in.as_deref(),
            language: page.language.as_deref(),
            since: page.since.as_deref(),
            aliases: &page.aliases,
            intro,
            body,
        }
    }
}

pub(crate) fn run(cli: &Cli, args: &ShowArgs) -> ExitCode {
    let index = match build_layered_index_with_overlay(cli.global.overlay.as_deref()) {
        Ok(idx) => idx,
        Err(msg) => {
            emit_index_failure(cli, &msg, &args.tool);
            return ExitCode::from(LoranExit::IndexNotBuilt.to_process_code());
        }
    };

    // Resolve the tldr cache: use the real `TldrCache` rooted under
    // `$XDG_CACHE_HOME/loran/tldr/extracted/` when one is available,
    // otherwise fall through with a `NoTldr` stub. We avoid an
    // expensive read at handler entry — `TldrCache::lookup` only
    // touches disk if `resolve_show_with_tldr` actually calls it.
    let tldr: Box<dyn TldrLookup> = match TldrCache::with_default_path() {
        Some(cache) => Box::new(cache),
        None => Box::new(NoTldr),
    };

    match resolve_show_with_tldr(&index, &args.tool, tldr.as_ref()) {
        ShowResult::IndexHit { page, intro, body } => emit_hit(cli, &page, intro, body),
        ShowResult::NoEntry { tool, hint } => emit_no_entry(cli, &tool, &hint),
    }
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
    let envelope = Envelope::new(command, ShowData::from_parts(page, intro, body));
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
