// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran new` — scaffold a new curated page in an overlay.
//!
//! Non-interactive mode (WP-P2.14): every required field is passed on
//! the command line; the handler validates the inputs via
//! [`loran_pages::Page::parse`], writes the rendered template to
//! `$XDG_DATA_HOME/loran/overlays/user/<category>/<tool>.md` atomically
//! (sibling temp + rename), and optionally execs `$EDITOR` on the
//! result. Interactive prompts and category autocomplete land in
//! WP-P2.15 once the TUI exists.
//!
//! Exit codes:
//! - 0 on success.
//! - 2 (`USAGE_ERROR`) on missing required flags or invalid frontmatter.
//! - 5 (`CONFLICT`) when the target file already exists (overlay author
//!   should `loran show` or edit it, not silently overwrite).
//! - 10 (`OVERLAY_WRITE_DENIED`) on permission failures along the
//!   `overlays/user/...` path.

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use loran_pages::Page;
use serde::Serialize;

use crate::cli::{Cli, Format, NewArgs, NewScope};
use crate::envelope::{Envelope, ErrorEnvelope, JsonEmitter};
use crate::exit::{ErrorContext, ExitCode as LoranExit};

/// Default page template, baked into the binary. On first run we copy
/// this to `$XDG_DATA_HOME/loran/templates/tool.md` so users can
/// customise it; the in-binary copy stays the source of truth for
/// every fresh `loran new` invocation that hasn't customised.
const BUILTIN_TEMPLATE: &str = include_str!("../../templates/tool.md");

#[derive(Serialize)]
struct NewData<'a> {
    tool: &'a str,
    category: &'a str,
    summary: &'a str,
    scope: &'static str,
    path: String,
    edited: bool,
}

pub(crate) fn run(cli: &Cli, args: &NewArgs) -> ExitCode {
    let category = match args.category.as_deref() {
        Some(c) => c.trim(),
        None => {
            return emit_usage(
                cli,
                args,
                "missing required --category flag",
                "loran new <tool> --category <slug> --summary <one-liner>",
            );
        }
    };

    let Some(summary) = args.summary.as_deref() else {
        return emit_usage(
            cli,
            args,
            "missing required --summary flag",
            "loran new <tool> --category <slug> --summary <one-liner>",
        );
    };

    let tool = args.tool.trim();
    if tool.is_empty() {
        return emit_usage(
            cli,
            args,
            "tool name is empty",
            "loran new <tool> --category <slug> --summary <one-liner>",
        );
    }

    let rendered = render_template(
        tool,
        category,
        summary,
        &args.replaces,
        &args.safe_alias_for,
    );

    // Validate before touching disk — a malformed page never lands.
    if let Err(err) = Page::parse(&rendered) {
        let code = LoranExit::PageParseError;
        emit_parse_failure(cli, args, &err.to_string(), code);
        return ExitCode::from(code.to_process_code());
    }

    let overlay_root = match overlay_root_for(args.scope) {
        Ok(root) => root,
        Err(msg) => {
            emit_overlay_denied(cli, args, &msg);
            return ExitCode::from(LoranExit::OverlayWriteDenied.to_process_code());
        }
    };

    let target = overlay_root.join(category).join(format!("{tool}.md"));

    if target.exists() {
        emit_conflict(cli, args, &target);
        return ExitCode::from(LoranExit::Conflict.to_process_code());
    }

    if let Err(err) = atomic_write(&target, &rendered) {
        emit_overlay_denied(cli, args, &err);
        return ExitCode::from(LoranExit::OverlayWriteDenied.to_process_code());
    }

    // Best-effort template copy. A failure here is non-fatal — the user
    // just doesn't get a customisable template on first run.
    if let Err(err) = seed_user_template() {
        tracing::debug!("user-template seed skipped: {err}");
    }

    let edited = if args.edit {
        open_editor(&target).is_ok()
    } else {
        false
    };

    emit_success(cli, args, tool, category, summary, &target, edited)
}

/// Resolve the overlay root for `scope`. The `User` scope returns
/// `$XDG_DATA_HOME/loran/overlays/user/`. The `Upstream` scope honours
/// `LORAN_UPSTREAM_PATH` (the configured Steelbore-publisher checkout)
/// until the proper config-file plumbing lands.
fn overlay_root_for(scope: NewScope) -> Result<PathBuf, String> {
    match scope {
        NewScope::User => dirs::data_dir()
            .map(|d| d.join("loran").join("overlays").join("user"))
            .ok_or_else(|| "no $XDG_DATA_HOME / data directory on this platform".to_owned()),
        NewScope::Upstream => match std::env::var("LORAN_UPSTREAM_PATH") {
            Ok(path) if !path.is_empty() => Ok(PathBuf::from(path).join("pages")),
            _ => Err(
                "`--scope=upstream` requires LORAN_UPSTREAM_PATH set to your \
                 Steelbore-publisher checkout (config-file support arrives in a \
                 later sub-phase)"
                    .to_owned(),
            ),
        },
    }
}

fn render_template(
    tool: &str,
    category: &str,
    summary: &str,
    replaces: &[String],
    safe_alias_for: &[String],
) -> String {
    let replaces_list = toml_string_array(replaces);
    let safe_alias_list = toml_string_array(safe_alias_for);
    BUILTIN_TEMPLATE
        .replace("{{name}}", tool)
        .replace("{{category}}", category)
        .replace("{{summary}}", &escape_toml_string(summary))
        .replace("{{replaces}}", &replaces_list)
        .replace("{{safe_alias_for}}", &safe_alias_list)
}

fn toml_string_array(items: &[String]) -> String {
    items
        .iter()
        .map(|s| format!("\"{}\"", escape_toml_string(s)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Escape the characters that would break a TOML double-quoted basic
/// string: backslash, double-quote, and control characters. Newlines
/// in the summary are illegal at the schema level anyway, but the
/// escape keeps the file machine-readable even if a user pastes one.
fn escape_toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn atomic_write(target: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }

    let staging = target.with_extension("md.tmp");
    // The tmp suffix collision is rare but worth covering: a previous
    // crashed `loran new` may have left one. Truncating on open is the
    // standard recovery.
    let mut file =
        fs::File::create(&staging).map_err(|e| format!("create {}: {e}", staging.display()))?;
    file.write_all(body.as_bytes())
        .map_err(|e| format!("write {}: {e}", staging.display()))?;
    file.sync_all()
        .map_err(|e| format!("fsync {}: {e}", staging.display()))?;
    drop(file);

    fs::rename(&staging, target).map_err(|e| {
        // Best-effort cleanup of the staging file on rename failure so
        // a retry doesn't trip the same suffix-already-exists path.
        let _ = fs::remove_file(&staging);
        format!("rename {} → {}: {e}", staging.display(), target.display())
    })
}

fn seed_user_template() -> Result<(), String> {
    let template_dir = dirs::data_dir()
        .map(|d| d.join("loran").join("templates"))
        .ok_or_else(|| "no data dir".to_owned())?;
    let template_path = template_dir.join("tool.md");
    if template_path.exists() {
        return Ok(());
    }
    fs::create_dir_all(&template_dir)
        .map_err(|e| format!("create {}: {e}", template_dir.display()))?;
    fs::write(&template_path, BUILTIN_TEMPLATE)
        .map_err(|e| format!("write {}: {e}", template_path.display()))?;
    Ok(())
}

fn open_editor(target: &Path) -> Result<(), String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .map_err(|_| "neither $EDITOR nor $VISUAL is set".to_owned())?;
    let status = Command::new(&editor)
        .arg(target)
        .status()
        .map_err(|e| format!("spawn `{editor}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("editor exited with status {status}"))
    }
}

fn emit_success(
    cli: &Cli,
    args: &NewArgs,
    tool: &str,
    category: &str,
    summary: &str,
    path: &Path,
    edited: bool,
) -> ExitCode {
    let scope = match args.scope {
        NewScope::User => "user",
        NewScope::Upstream => "upstream",
    };
    match cli.output_format() {
        Format::Json => {
            let envelope = Envelope::new(
                format!("loran new {tool}"),
                NewData {
                    tool,
                    category,
                    summary,
                    scope,
                    path: path.display().to_string(),
                    edited,
                },
            );
            let _ = JsonEmitter::stdio().emit_data(&envelope);
        }
        Format::Human => {
            println!("created {}", path.display());
            if edited {
                println!("editor session completed");
            } else if args.edit {
                eprintln!("note: editor failed to launch — file is on disk regardless");
            }
        }
    }
    ExitCode::from(0)
}

fn emit_usage(cli: &Cli, args: &NewArgs, msg: &str, hint: &str) -> ExitCode {
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                LoranExit::UsageError.name(),
                LoranExit::UsageError.numeric(),
                msg,
                hint,
                format!("loran new {}", args.tool),
                None,
            );
            let _ = JsonEmitter::stdio().emit_error(&envelope);
        }
        Format::Human => {
            eprintln!("error: {msg}");
            eprintln!("  hint: {hint}");
        }
    }
    ExitCode::from(LoranExit::UsageError.to_process_code())
}

fn emit_parse_failure(cli: &Cli, args: &NewArgs, msg: &str, code: LoranExit) {
    let ctx = ErrorContext::with_tool(&args.tool);
    let hint = code.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                msg,
                &hint,
                format!("loran new {}", args.tool),
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

fn emit_conflict(cli: &Cli, args: &NewArgs, target: &Path) {
    let code = LoranExit::Conflict;
    let ctx = ErrorContext::with_tool(&args.tool);
    let hint = code.hint(&ctx);
    let msg = format!(
        "overlay file already exists: {} — edit it directly or remove it first",
        target.display()
    );
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                &msg,
                &hint,
                format!("loran new {}", args.tool),
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

fn emit_overlay_denied(cli: &Cli, args: &NewArgs, msg: &str) {
    let code = LoranExit::OverlayWriteDenied;
    let ctx = ErrorContext::with_tool(&args.tool);
    let hint = code.hint(&ctx);
    match cli.output_format() {
        Format::Json => {
            let envelope = ErrorEnvelope::new(
                code.name(),
                code.numeric(),
                msg,
                &hint,
                format!("loran new {}", args.tool),
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
