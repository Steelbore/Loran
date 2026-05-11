// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran schema` — JSON Schema emission. Placeholder in Phase 1.
//!
//! In Phase 1 this emits a deliberately-marked-placeholder JSON
//! document that documents the `Page` shape only. The full schema
//! (every sub-command's data shape + exit codes + envelope) lands in
//! Phase 3 alongside the MCP surface.

use std::process::ExitCode;

use serde_json::json;

use crate::cli::{Cli, SchemaArgs};
use crate::envelope::JsonEmitter;

pub(crate) fn run(_cli: &Cli, _args: &SchemaArgs) -> ExitCode {
    let placeholder = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://Loran.Steelbore.com/schema/page.json",
        "title": "Loran Page (placeholder schema)",
        "description": "Phase 1 placeholder. The full schema covering every \
                        Loran data type, exit code, and envelope shape lands \
                        in Phase 3 alongside the MCP surface.",
        "meta": {
            "placeholder": true,
            "phase": "ingot",
            "covers": ["loran_pages::Page (frontmatter)"]
        },
        "type": "object",
        "required": ["name", "category", "summary"],
        "properties": {
            "name":           { "type": "string", "description": "Canonical binary name." },
            "category":       { "type": "string", "description": "Category slug (slash-tolerant)." },
            "summary":        { "type": "string", "maxLength": 120 },
            "replaces":       { "type": "array", "items": { "type": "string" } },
            "safe_alias_for": { "type": "array", "items": { "type": "string" } },
            "pairs_with":     { "type": "array", "items": { "type": "string" } },
            "official":       { "type": ["string", "null"] },
            "tldr_page":      { "type": ["string", "null"] },
            "tags":           { "type": "array", "items": { "type": "string" } },
            "written_in":     { "type": ["string", "null"] },
            "language":       { "type": ["string", "null"] },
            "since":          { "type": ["string", "null"] },
            "aliases":        { "type": "array", "items": { "type": "string" } }
        }
    });

    // Schema output bypasses the standard Envelope by design — it is a
    // schema document, not a `data` payload. Future revisions may wrap
    // it in an envelope alongside metadata.
    let rendered =
        serde_json::to_string_pretty(&placeholder).unwrap_or_else(|_| placeholder.to_string());
    println!("{rendered}");

    // Silence the unused-import lint until JsonEmitter is the wrapper
    // used in Phase 3.
    let _ = std::any::type_name::<JsonEmitter<std::io::Stdout, std::io::Stderr>>();

    ExitCode::from(0)
}
