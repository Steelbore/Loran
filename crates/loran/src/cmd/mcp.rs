// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran mcp` — run the read-only Model Context Protocol server on
//! stdio (WP-P3.03).
//!
//! Wires the layered index (bundled + overlays) into
//! [`loran_mcp::serve_stdio`], blocking on stdin until the client
//! closes its end. Logging goes to stderr — stdout is reserved for
//! JSON-RPC traffic, which is why we deliberately don't use the
//! `Envelope` / `JsonEmitter` paths here.

use std::io::{BufReader, stdin, stdout};
use std::process::ExitCode;

use loran_mcp::{Handler, serve_stdio};

use crate::cli::{Cli, McpArgs};
use crate::exit::ExitCode as LoranExit;
use crate::index_loader::build_layered_index_with_overlay;

pub(crate) fn run(cli: &Cli, _args: &McpArgs) -> ExitCode {
    let index = match build_layered_index_with_overlay(cli.global.overlay.as_deref()) {
        Ok(idx) => idx,
        Err(msg) => {
            eprintln!("loran mcp: index unavailable: {msg}");
            return ExitCode::from(LoranExit::IndexNotBuilt.to_process_code());
        }
    };

    let handler = Handler::new(index);
    let reader = BufReader::new(stdin().lock());
    let writer = stdout().lock();

    if let Err(err) = serve_stdio(&handler, reader, writer) {
        eprintln!("loran mcp: stdio loop failed: {err}");
        return ExitCode::from(LoranExit::GeneralError.to_process_code());
    }
    ExitCode::from(0)
}
