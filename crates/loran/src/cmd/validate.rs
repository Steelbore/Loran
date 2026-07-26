// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran validate` — walk every overlay root and validate every page.
//!
//! Spec §4.1 / FR-035: CI-friendly validator. Walks the three on-disk
//! roots in [`OverlayRoot`] order, parses each `.md` as either a full
//! [`Page`] or a partial [`OverlayPage`], and reports the first parse
//! failure per file (path + line + code + message).
//!
//! Exit code:
//! - 0 if every page validated successfully (or if no overlay root
//!   exists on disk yet — that's the fresh-install case).
//! - `PAGE_PARSE_ERROR` (8) if any single page failed.
//!
//! Bundled pages are not validated at runtime: the build script
//! already calls `Page::parse` on every one of them, so a broken
//! bundled page would fail to compile.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use loran_index::detect_distro_id;
use loran_pages::{OverlayPage, Page, PageError};
use serde::Serialize;
use walkdir::WalkDir;

use crate::cli::{Cli, Format, ValidateArgs};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

/// Layer-strictness mode. The upstream root must contain full pages
/// only; overlay roots accept either a full [`Page`] or a partial
/// [`OverlayPage`].
#[derive(Copy, Clone, Debug)]
enum LayerKind {
    Upstream,
    Overlay,
}

#[derive(Serialize, Debug)]
struct ValidationError {
    file: String,
    /// 1-based line number where the parser stopped. Best-effort: TOML
    /// errors carry exact spans; schema-level errors fall back to 1.
    line: u32,
    code: &'static str,
    message: String,
    layer: &'static str,
}

#[derive(Serialize, Debug)]
struct ValidateData {
    valid: u32,
    invalid: u32,
    errors: Vec<ValidationError>,
}

pub(crate) fn run(cli: &Cli, args: &ValidateArgs) -> ExitCode {
    // Explicit `loran validate <ROOT>`: validate a single tree as
    // upstream-strict (full pages only). Lets CI gate a pages repo
    // without provisioning `$XDG_DATA_HOME`. Otherwise fall back to
    // walking the on-disk overlay roots.
    let roots: Vec<(&'static str, PathBuf, LayerKind)> = if let Some(root) = &args.root {
        if !root.is_dir() {
            emit_bad_root(cli, root);
            return ExitCode::from(LoranExit::UsageError.to_process_code());
        }
        vec![("upstream", root.clone(), LayerKind::Upstream)]
    } else {
        let Some(data_dir) = loran_core::data_home() else {
            emit_no_data_dir(cli);
            return ExitCode::from(LoranExit::PermissionDenied.to_process_code());
        };
        let loran_root = data_dir.join("loran");
        let distro = active_distro();
        vec![
            ("upstream", loran_root.join("pages"), LayerKind::Upstream),
            (
                "distro",
                loran_root.join("overlays").join(&distro),
                LayerKind::Overlay,
            ),
            (
                "user",
                loran_root.join("overlays").join("user"),
                LayerKind::Overlay,
            ),
        ]
    };

    let mut data = ValidateData {
        valid: 0,
        invalid: 0,
        errors: Vec::new(),
    };

    for (layer_name, root, kind) in &roots {
        validate_root(layer_name, *kind, root, &mut data);
    }

    let exit_code = if data.invalid == 0 {
        LoranExit::Success
    } else {
        LoranExit::PageParseError
    };

    emit(cli, &data);
    ExitCode::from(exit_code.to_process_code())
}

/// Active-distro overlay name, honouring the `LORAN_DISTRO_OVERRIDE`
/// env var so tests can pin a specific layer without writing to
/// `/etc/os-release`.
fn active_distro() -> String {
    if let Ok(override_id) = std::env::var("LORAN_DISTRO_OVERRIDE") {
        if !override_id.is_empty() {
            return override_id;
        }
    }
    detect_distro_id()
}

fn validate_root(layer: &'static str, kind: LayerKind, root: &Path, out: &mut ValidateData) {
    if !root.is_dir() {
        return;
    }

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            entry
                .file_name()
                .to_str()
                .is_none_or(|n| !n.starts_with('.'))
        });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                out.invalid = out.invalid.saturating_add(1);
                let file = err
                    .path()
                    .map_or_else(|| "<unknown>".to_owned(), |p| p.display().to_string());
                out.errors.push(ValidationError {
                    file,
                    line: 1,
                    code: "IO_ERROR",
                    message: err.to_string(),
                    layer,
                });
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        match fs::read_to_string(path) {
            Ok(text) => check_one_file(layer, kind, path, &text, out),
            Err(err) => {
                out.invalid = out.invalid.saturating_add(1);
                out.errors.push(ValidationError {
                    file: path.display().to_string(),
                    line: 1,
                    code: "IO_ERROR",
                    message: err.to_string(),
                    layer,
                });
            }
        }
    }
}

fn check_one_file(
    layer: &'static str,
    kind: LayerKind,
    path: &Path,
    text: &str,
    out: &mut ValidateData,
) {
    let full = Page::parse(text);
    if full.is_ok() {
        out.valid = out.valid.saturating_add(1);
        return;
    }

    let original_err = full.unwrap_err();

    // Upstream pages are authoritative; partial overlays are not
    // accepted there. Overlay roots accept either form, so for the
    // missing-required-field case try the partial-overlay parser as a
    // fallback.
    if matches!(kind, LayerKind::Overlay) {
        if let PageError::MissingField(field) = &original_err {
            if *field == "category" || *field == "summary" {
                match OverlayPage::parse(text) {
                    Ok(_) => {
                        out.valid = out.valid.saturating_add(1);
                        return;
                    }
                    Err(overlay_err) => {
                        push_error(out, layer, path, text, &overlay_err);
                        return;
                    }
                }
            }
        }
    }

    push_error(out, layer, path, text, &original_err);
}

fn push_error(
    out: &mut ValidateData,
    layer: &'static str,
    path: &Path,
    text: &str,
    err: &PageError,
) {
    out.invalid = out.invalid.saturating_add(1);
    let (code, line) = classify(err, text);
    out.errors.push(ValidationError {
        file: path.display().to_string(),
        line,
        code,
        message: err.to_string(),
        layer,
    });
}

fn classify(err: &PageError, text: &str) -> (&'static str, u32) {
    match err {
        PageError::MissingFrontmatter => ("MISSING_FRONTMATTER", 1),
        PageError::InvalidToml(toml_err) => {
            let line = toml_err
                .span()
                .map_or(1, |span| line_from_offset(text, span.start));
            ("INVALID_TOML", line)
        }
        PageError::MissingField(_) => ("MISSING_FIELD", 1),
        PageError::SummaryTooLong { .. } => ("SUMMARY_TOO_LONG", line_of_key(text, "summary")),
        PageError::InvalidSafeAliasFor(_) => (
            "INVALID_SAFE_ALIAS_FOR",
            line_of_key(text, "safe_alias_for"),
        ),
        PageError::InvalidCategory { .. } => ("INVALID_CATEGORY", line_of_key(text, "category")),
        PageError::InvalidTldrPage { .. } => ("INVALID_TLDR_PAGE", line_of_key(text, "tldr_page")),
    }
}

/// 1-based line number for a byte offset within `text`.
fn line_from_offset(text: &str, offset: usize) -> u32 {
    let clamped = offset.min(text.len());
    // Frontmatter blocks are short — the byte slice never exceeds a
    // few KiB. Pulling in `bytecount` for SIMD newline counting is not
    // worth the dependency cost.
    #[allow(clippy::naive_bytecount)]
    let count = text.as_bytes()[..clamped]
        .iter()
        .filter(|b| **b == b'\n')
        .count();
    u32::try_from(count + 1).unwrap_or(u32::MAX)
}

/// Best-effort: find the first line whose trimmed text starts with
/// `<key> =` or `<key>=`, and return its 1-based line number. Falls
/// back to 1 if not found (e.g. the field's source lives in an
/// included file we can't see).
fn line_of_key(text: &str, key: &str) -> u32 {
    for (idx, line) in text.lines().enumerate() {
        let stripped = line.trim_start();
        if let Some(after) = stripped.strip_prefix(key) {
            if after.trim_start().starts_with('=') {
                return u32::try_from(idx + 1).unwrap_or(u32::MAX);
            }
        }
    }
    1
}

fn emit(cli: &Cli, data: &ValidateData) {
    match cli.output_format() {
        Format::Json => {
            let envelope = Envelope::new("loran validate", data);
            let _ = JsonEmitter::stdio().emit_data(&envelope);
        }
        Format::Human => emit_human(data),
    }
}

fn emit_human(data: &ValidateData) {
    if data.errors.is_empty() {
        println!("validate: {} pages OK, no errors", data.valid);
        return;
    }
    for err in &data.errors {
        eprintln!(
            "{file}:{line}: [{layer}] {code}: {msg}",
            file = err.file,
            line = err.line,
            layer = err.layer,
            code = err.code,
            msg = err.message
        );
    }
    eprintln!();
    eprintln!(
        "validate: {valid} OK, {invalid} failed",
        valid = data.valid,
        invalid = data.invalid
    );
}

fn emit_no_data_dir(cli: &Cli) {
    let code = LoranExit::PermissionDenied;
    let ctx = ErrorContext::empty();
    let hint = code.hint(&ctx);
    let msg = "no $XDG_DATA_HOME / data directory available on this platform";
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                msg,
                &hint,
                "loran validate",
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

fn emit_bad_root(cli: &Cli, root: &Path) {
    let code = LoranExit::UsageError;
    let msg = format!("validate root is not a directory: {}", root.display());
    let hint = "pass a directory of pages, e.g. `loran validate pages/`";
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                &msg,
                hint,
                "loran validate",
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
