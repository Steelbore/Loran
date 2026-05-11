// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![forbid(unsafe_code)]

//! Loran xtask runner.
//!
//! Invoke via `cargo xtask <subcommand>` (aliased in `.cargo/config.toml`).
//!
//! Phase 0 subcommands:
//!
//! - `check-spdx` — verifies every `.rs` and `Cargo.toml` file carries the
//!   two-line SPDX header. Walks the workspace from the crate's `CARGO_MANIFEST_DIR`
//!   parent (i.e. the workspace root). Skips `target/` and `.git/`.

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, anyhow, bail};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let subcommand = args.get(1).map(String::as_str);

    let result = match subcommand {
        Some("check-spdx") => check_spdx(),
        Some("help" | "--help" | "-h") | None => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Some(other) => Err(anyhow!("unknown xtask subcommand: {other}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask: error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "Loran xtask helpers.\n\n\
        USAGE:\n    cargo xtask <subcommand>\n\n\
        SUBCOMMANDS:\n\
            check-spdx    Verify SPDX headers on every .rs and Cargo.toml file.\n\
            help          Show this help.\n"
    );
}

fn workspace_root() -> Result<PathBuf> {
    // CARGO_MANIFEST_DIR points at xtask/; its parent is the workspace root.
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .context("CARGO_MANIFEST_DIR not set; run via `cargo xtask <cmd>`")?;
    let xtask_dir = PathBuf::from(manifest_dir);
    xtask_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("xtask manifest dir has no parent"))
}

fn check_spdx() -> Result<()> {
    let root = workspace_root()?;
    let mut missing: Vec<PathBuf> = Vec::new();
    let mut checked: usize = 0;

    walk(&root, &mut |path| {
        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            return Ok(());
        };
        let ext = path.extension().and_then(OsStr::to_str);
        let interesting = ext == Some("rs") || name == "Cargo.toml";
        if !interesting {
            return Ok(());
        }
        checked += 1;
        let body =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        if !has_spdx_header(&body) {
            missing.push(path.to_path_buf());
        }
        Ok(())
    })?;

    if !missing.is_empty() {
        eprintln!(
            "check-spdx: {missing} of {checked} files are missing SPDX headers:",
            missing = missing.len(),
            checked = checked
        );
        for path in &missing {
            eprintln!("  {}", path.display());
        }
        bail!("SPDX header check failed");
    }

    println!("check-spdx: ok ({checked} files)");
    Ok(())
}

fn has_spdx_header(body: &str) -> bool {
    // First ~20 lines must contain both the licence identifier and a
    // copyright statement. We don't pin to lines 1-2 exactly because
    // shebangs, encoding declarations, or HTML comment wrappers may
    // shift the headers down.
    let head: String = body.lines().take(20).collect::<Vec<_>>().join("\n");
    head.contains("SPDX-License-Identifier: GPL-3.0-or-later")
        && head.contains("SPDX-FileCopyrightText:")
}

/// Recursively walk `root`, calling `visit` on every regular file.
///
/// Skips `target/`, `.git/`, and any hidden directory.
fn walk<F>(root: &Path, visit: &mut F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    let entries =
        fs::read_dir(root).with_context(|| format!("reading directory {}", root.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if path.is_dir() {
            if name_str == "target" || name_str == ".git" || name_str.starts_with('.') {
                continue;
            }
            walk(&path, visit)?;
        } else if path.is_file() {
            visit(&path)?;
        }
    }
    Ok(())
}
