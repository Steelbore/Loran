// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran describe` — the CLI Standard's describe manifest for agents.
//!
//! Always JSON-shaped (the CLI Standard §4); `--format human` produces the same
//! envelope pretty-printed, not a different shape.

use std::process::ExitCode;

use serde::Serialize;

use crate::cli::{Cli, DescribeArgs};
use crate::envelope::{Envelope, JsonEmitter};
use crate::exit::ExitCode as LoranExit;

const TOOL_NAME: &str = "loran";
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct DescribeData {
    tool: &'static str,
    version: &'static str,
    commands: Vec<CommandManifest>,
    global_flags: Vec<&'static str>,
    exit_codes: Vec<ExitCodeManifest>,
}

#[derive(Serialize)]
struct CommandManifest {
    name: &'static str,
    summary: &'static str,
    capability_tags: Vec<&'static str>,
}

#[derive(Serialize)]
struct ExitCodeManifest {
    name: &'static str,
    numeric: i32,
}

pub(crate) fn run(_cli: &Cli, _args: &DescribeArgs) -> ExitCode {
    let data = DescribeData {
        tool: TOOL_NAME,
        version: TOOL_VERSION,
        commands: command_manifest(),
        global_flags: global_flag_manifest(),
        exit_codes: exit_code_manifest(),
    };
    let envelope = Envelope::new("loran describe", data);
    let _ = JsonEmitter::stdio().emit_data(&envelope);
    ExitCode::from(0)
}

/// Capability-tag taxonomy per `spacecraft-agentic-cli` §4:
///
/// - `read-only` — does not modify any state on disk or network.
/// - `network` — performs outbound network I/O.
/// - `subprocess` — spawns external binaries.
/// - `write` — mutates files under the user's data dirs.
fn command_manifest() -> Vec<CommandManifest> {
    vec![
        CommandManifest {
            name: "list",
            summary: "List tools in the catalog (filterable).",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "show",
            summary: "Show the curated page for a tool (curated-or-fail).",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "help",
            summary: "Capture and render a binary's --help output (de-themed).",
            capability_tags: vec!["read-only", "subprocess"],
        },
        CommandManifest {
            name: "find",
            summary: "Reverse lookup — which tool replaces a legacy name?",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "search",
            summary: "Fuzzy search across name, summary, replaces, and tags.",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "categories",
            summary: "List categories with counts.",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "new",
            summary: "Scaffold a new curated page in the user overlay.",
            capability_tags: vec!["write"],
        },
        CommandManifest {
            name: "update",
            summary: "Refresh upstream tarballs (signed) and rebuild the index.",
            capability_tags: vec!["network", "write"],
        },
        CommandManifest {
            name: "validate",
            summary: "Validate every page against the frontmatter schema.",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "schema",
            summary: "Emit JSON Schema for the Loran data model.",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "describe",
            summary: "Emit the CLI Standard's describe manifest for agents.",
            capability_tags: vec!["read-only"],
        },
        CommandManifest {
            name: "mcp",
            summary: "Run as a read-only MCP server over stdio.",
            capability_tags: vec!["read-only", "network"],
        },
    ]
}

fn global_flag_manifest() -> Vec<&'static str> {
    vec![
        "--json",
        "--format",
        "--fields",
        "--dry-run",
        "--verbose",
        "--quiet",
        "--color",
        "--no-color",
        "--help",
        "--version",
        "--absolute-time",
        "--print0",
        "--yes",
    ]
}

fn exit_code_manifest() -> Vec<ExitCodeManifest> {
    LoranExit::all()
        .iter()
        .map(|code| ExitCodeManifest {
            name: code.name(),
            numeric: code.numeric(),
        })
        .collect()
}
