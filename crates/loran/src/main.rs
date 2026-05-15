// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![forbid(unsafe_code)]

//! Loran CLI binary entry point.
//!
//! Phase 1B scaffolding: parses every SFRS global flag, dispatches to
//! one of twelve sub-commands, and surfaces a custom `--version` /
//! `--help` footer per Spacecraft Software Standard v1.1 §13.2. Sub-command
//! handlers are stubs that emit a "not yet implemented" notice and
//! exit with [`ExitCode::NotYetImplemented`]; real implementations land
//! in Sub-phases 1C–1D per `loran-plan-v0_1.md`.

mod agent;
mod cli;
mod cmd;
mod color;
mod envelope;
mod exit;
mod index_loader;
mod logging;
mod summary;
mod version;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Color resolution must happen before any output writes so the
    // (currently bare) `tracing-subscriber` and the `not yet
    // implemented` notices both honour `--no-color` / `NO_COLOR`.
    let _color = color::resolve(&cli);
    logging::init(&cli);

    if cli.version {
        return version::emit(&cli);
    }

    if let Some(cmd) = cli.command.as_ref() {
        match cmd {
            Command::Categories(args) => cmd::categories::run(&cli, args),
            Command::Describe(args) => cmd::describe::run(&cli, args),
            Command::Find(args) => cmd::find::run(&cli, args),
            Command::Help(args) => cmd::help::run(&cli, args),
            Command::List(args) => cmd::list::run(&cli, args),
            Command::New(args) => cmd::new::run(&cli, args),
            Command::Schema(args) => cmd::schema::run(&cli, args),
            Command::Search(args) => cmd::search::run(&cli, args),
            Command::Show(args) => cmd::show::run(&cli, args),
            Command::Update(args) => cmd::update::run(&cli, args),
            Command::Validate(args) => cmd::validate::run(&cli, args),
            Command::Mcp(args) => cmd::mcp::run(&cli, args),
        }
    } else {
        // No sub-command + no --version: launch the TUI when running
        // interactively, otherwise fall back to a usage hint so pipes
        // and agent runners get something readable.
        if should_launch_tui(&cli) {
            launch_tui(&cli)
        } else {
            eprintln!(
                "loran: no sub-command given. Try `loran --help` for the \
                 full surface, `loran list` for the catalog, or run in a \
                 TTY without --json to open the interactive browser."
            );
            ExitCode::from(0)
        }
    }
}

/// Decide whether to launch the TUI for the no-subcommand path.
///
/// Mirrors the SFRS §5 agent cascade: explicit JSON / agent env / pipe
/// all disqualify the TUI. Stdout must be a TTY for the curses-style
/// surface to make sense.
fn should_launch_tui(cli: &Cli) -> bool {
    use is_terminal::IsTerminal as _;

    if cli.global.json || cli.global.format == Some(cli::Format::Json) {
        return false;
    }
    for var in &[
        "AI_AGENT",
        "AGENT",
        "CI",
        "CLAUDECODE",
        "CURSOR_AGENT",
        "GEMINI_CLI",
    ] {
        if std::env::var(var).is_ok_and(|v| !v.is_empty()) {
            return false;
        }
    }
    std::io::stdout().is_terminal()
}

fn launch_tui(cli: &Cli) -> ExitCode {
    let index = match index_loader::build_layered_index_with_overlay(cli.global.overlay.as_deref())
    {
        Ok(idx) => idx,
        Err(msg) => {
            eprintln!("loran: index unavailable: {msg}");
            return ExitCode::from(6);
        }
    };
    let mut app = loran_tui::App::from_env(index);
    if let Err(err) = loran_tui::run(&mut app) {
        eprintln!("loran: tui failed: {err}");
        return ExitCode::from(1);
    }
    ExitCode::from(0)
}
