// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! MCP tool catalogue — five read-only verbs.
//!
//! `tools/list` advertises every entry in [`allowed_tools`]; `tools/call`
//! refuses to dispatch any name not present in this list. Write verbs
//! (`update`, `new`, `validate`) and the subprocess-spawning `help`
//! verb are absent by design (Spec §12.2).

use serde::Serialize;
use serde_json::{Value, json};

/// One advertised MCP tool — name, description, and input schema.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// The five read-only verbs Loran exposes over MCP. Order is stable
/// — matches Spec §12.2's enumeration.
#[must_use]
pub(crate) fn allowed_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list",
            description: "List every page in the merged catalog. Optionally \
                          filter by `category`, `replaces`, or `safe_alias_for`.",
            input_schema: list_input_schema(),
        },
        ToolDefinition {
            name: "show",
            description: "Return the curated page for a tool by canonical \
                          name. Returns a NOT_FOUND envelope when no entry \
                          exists; never falls back to live `--help`.",
            input_schema: show_input_schema(),
        },
        ToolDefinition {
            name: "find",
            description: "Reverse-lookup: which Loran-blessed tool replaces \
                          a legacy binary name? Set `safe_alias_only=true` \
                          for the subset that can be aliased without \
                          breaking common-case scripts.",
            input_schema: find_input_schema(),
        },
        ToolDefinition {
            name: "search",
            description: "Fuzzy match across name / summary / replaces / \
                          tags via nucleo-matcher. Returns scored hits.",
            input_schema: search_input_schema(),
        },
        ToolDefinition {
            name: "categories",
            description: "Registry of every Loran category with title, \
                          description, and live page count.",
            input_schema: categories_input_schema(),
        },
    ]
}

/// `true` when `name` is one of the read-only verbs. Anything else
/// (`update`, `new`, `validate`, `help`, …) is rejected at the
/// `tools/call` boundary.
#[must_use]
pub(crate) fn is_allowed(name: &str) -> bool {
    allowed_tools().iter().any(|t| t.name == name)
}

fn list_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "category": { "type": "string", "description": "Filter by category slug." },
            "replaces": { "type": "string", "description": "Filter to pages whose `replaces` includes this legacy tool." },
            "safe_alias_for": { "type": "string", "description": "Filter to pages whose `safe_alias_for` includes this legacy tool." }
        },
        "additionalProperties": false
    })
}

fn show_input_schema() -> Value {
    json!({
        "type": "object",
        "required": ["tool"],
        "properties": {
            "tool": { "type": "string", "description": "Canonical tool name (e.g. `eza`)." }
        },
        "additionalProperties": false
    })
}

fn find_input_schema() -> Value {
    json!({
        "type": "object",
        "required": ["legacy"],
        "properties": {
            "legacy": { "type": "string", "description": "Legacy binary name (e.g. `ls`, `cat`, `grep`)." },
            "safe_alias_only": { "type": "boolean", "description": "Strict mode: only entries where alias=modern won't break scripts.", "default": false }
        },
        "additionalProperties": false
    })
}

fn search_input_schema() -> Value {
    json!({
        "type": "object",
        "required": ["query"],
        "properties": {
            "query": { "type": "string", "description": "Free-text fuzzy query." }
        },
        "additionalProperties": false
    })
}

fn categories_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::{allowed_tools, is_allowed};

    #[test]
    fn exactly_five_read_only_verbs() {
        let tools = allowed_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name).collect();
        assert_eq!(names, vec!["list", "show", "find", "search", "categories"]);
    }

    #[test]
    fn write_verbs_are_not_allowed() {
        for verb in ["update", "new", "validate", "help", "schema", "describe"] {
            assert!(!is_allowed(verb), "{verb} must NOT be exposed via MCP");
        }
    }

    #[test]
    fn every_tool_has_a_non_empty_description() {
        for tool in allowed_tools() {
            assert!(!tool.description.is_empty(), "{} desc empty", tool.name);
        }
    }
}
