// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![forbid(unsafe_code)]

//! Loran CLI binary entry point.
//!
//! Phase 1B scaffolding: parses every SFRS global flag, dispatches to
//! one of twelve sub-commands, and surfaces a custom `--version` /
//! `--help` footer per Steelbore Standard v1.1 §13.2. Sub-command
//! handlers are stubs that emit a "not yet implemented" notice and
//! exit with [`ExitCode::NotYetImplemented`]; real implementations land
//! in Sub-phases 1C–1D per `loran-plan-v0_1.md`.

mod agent;
mod cli;
mod cmd;
mod color;
mod envelope;
mod exit;
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
            Command::Schema(args) => cmd::schema::run(&cli, args),
            Command::Search(args) => cmd::search::run(&cli, args),
            Command::Show(args) => cmd::show::run(&cli, args),
            _ => {
                let name = cmd.name();
                eprintln!(
                    "loran {name}: not yet implemented in this Phase 1 milestone. \
                     See `loran-plan-v0_1.md` WP-P1.11 onwards."
                );
                ExitCode::from(1)
            }
        }
    } else {
        // No sub-command + no --version: print usage hint and exit 0.
        // Phase 2 will replace this with the TUI default view.
        eprintln!(
            "loran: no sub-command given. Try `loran --help` for the \
             full surface, or `loran list` once Phase 1 sub-commands \
             land."
        );
        ExitCode::from(0)
    }
}
