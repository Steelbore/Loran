// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran list` — filterable catalog listing.

use std::io::Write as _;
use std::process::ExitCode;

use loran_pages::Page;

use crate::cli::{Cli, Format, ListArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};
use crate::index_loader::build_layered_index_with_overlay;
use crate::summary::PageSummary;
use loran_index::Index;

pub(crate) fn run(cli: &Cli, args: &ListArgs) -> ExitCode {
    let index = match build_layered_index_with_overlay(cli.global.overlay.as_deref()) {
        Ok(idx) => idx,
        Err(msg) => {
            emit_index_failure(cli, &msg);
            return ExitCode::from(LoranExit::IndexNotBuilt.to_process_code());
        }
    };

    let mut pages: Vec<&Page> = filter_pages(&index, args).collect();
    pages.sort_by(|a, b| a.name.cmp(&b.name));

    match cli.output_format() {
        Format::Json => emit_json(&pages),
        Format::Human => emit_text(cli, &pages),
    }

    ExitCode::from(0)
}

fn filter_pages<'a>(index: &'a Index, args: &ListArgs) -> impl Iterator<Item = &'a Page> {
    index.all().filter(|page| {
        if let Some(cat) = &args.category
            && page.category != *cat
        {
            return false;
        }
        if let Some(legacy) = &args.replaces
            && !page.replaces.iter().any(|r| r == legacy)
        {
            return false;
        }
        if let Some(legacy) = &args.safe_alias_for
            && !page.safe_alias_for.iter().any(|s| s == legacy)
        {
            return false;
        }
        true
    })
}

fn emit_json(pages: &[&Page]) {
    let summaries: Vec<PageSummary<'_>> = pages.iter().map(|p| PageSummary::from(*p)).collect();
    let envelope = Envelope::new("loran list", summaries);
    let _ = JsonEmitter::stdio().emit_data(&envelope);
}

fn emit_text(cli: &Cli, pages: &[&Page]) {
    let fields = resolve_fields(cli);
    let mut stdout = std::io::stdout().lock();
    for page in pages {
        let columns: Vec<String> = fields.iter().map(|f| field_value(page, f)).collect();
        let _ = writeln!(stdout, "{}", columns.join("\t"));
    }
}

/// Resolve which columns to emit. Default: name, category, summary.
/// `--fields name,replaces` overrides.
fn resolve_fields(cli: &Cli) -> Vec<String> {
    if cli.global.fields.is_empty() {
        vec!["name".into(), "category".into(), "summary".into()]
    } else {
        cli.global.fields.clone()
    }
}

fn field_value(page: &Page, field: &str) -> String {
    match field {
        "name" => page.name.clone(),
        "category" => page.category.clone(),
        "summary" => page.summary.clone(),
        "replaces" => page.replaces.join(","),
        "safe_alias_for" => page.safe_alias_for.join(","),
        "pairs_with" => page.pairs_with.join(","),
        "tags" => page.tags.join(","),
        "written_in" => page.written_in.clone().unwrap_or_default(),
        "official" => page.official.clone().unwrap_or_default(),
        "since" => page.since.clone().unwrap_or_default(),
        "aliases" => page.aliases.join(","),
        // Unknown field name → empty column (don't fail; lets agents
        // iterate the catalog defensively).
        _ => String::new(),
    }
}

fn emit_index_failure(cli: &Cli, msg: &str) {
    let ctx = ErrorContext::empty();
    let hint = LoranExit::IndexNotBuilt.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                LoranExit::IndexNotBuilt.name(),
                LoranExit::IndexNotBuilt.numeric(),
                msg,
                &hint,
                "loran list",
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
