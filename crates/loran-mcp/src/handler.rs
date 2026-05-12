// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! MCP method-dispatch handler. Pure — no I/O.
//!
//! Given a parsed [`JsonRpcRequest`] and a built [`Index`], returns a
//! ready-to-serialise [`JsonRpcResponse`]. The server loop owns
//! stdin/stdout and threads its requests through here.

// Dispatch uniformity: every tool_* method returns `Result<Value, _>`
// so the call site can `?` regardless of whether the underlying
// resolver is fallible. The lints below would suggest dropping the
// `Result` from always-Ok methods, which would break that uniformity.
#![allow(clippy::unnecessary_wraps, clippy::unused_self)]

use loran_core::{bundled_categories, resolve_find, resolve_search, resolve_show};
use loran_index::Index;
use loran_pages::Page;
use serde_json::{Value, json};

use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, error_codes};
use crate::tools::{allowed_tools, is_allowed};

/// Dispatch a parsed JSON-RPC request against `index`.
///
/// Returns `None` for notifications (requests without an `id`); the
/// server loop should drop those silently per JSON-RPC 2.0.
#[derive(Debug)]
pub struct Handler {
    index: Index,
    server_name: String,
    server_version: String,
}

impl Handler {
    /// Build a handler with a default `serverInfo` block. The `version`
    /// is the Loran crate version pulled from Cargo at build time.
    #[must_use]
    pub fn new(index: Index) -> Self {
        Self {
            index,
            server_name: "loran".to_owned(),
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    /// Process one request. Returns `None` for notifications.
    #[must_use]
    pub fn handle(&self, req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id.clone()?;
        let resp = match req.method.as_str() {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&req.params),
            "ping" => Ok(json!({})),
            other => Err(JsonRpcError::new(
                error_codes::METHOD_NOT_FOUND,
                format!("unknown method `{other}`"),
            )),
        };
        Some(match resp {
            Ok(result) => JsonRpcResponse::ok(id, result),
            Err(error) => JsonRpcResponse::err(id, error),
        })
    }

    fn handle_initialize(&self) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name":    self.server_name,
                "version": self.server_version,
            }
        }))
    }

    fn handle_tools_list(&self) -> Result<Value, JsonRpcError> {
        Ok(json!({
            "tools": allowed_tools(),
        }))
    }

    fn handle_tools_call(&self, params: &Value) -> Result<Value, JsonRpcError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::new(error_codes::INVALID_PARAMS, "`name` is required"))?;

        if !is_allowed(name) {
            return Err(JsonRpcError::new(
                error_codes::WRITE_VERB_REJECTED,
                format!(
                    "verb `{name}` is not exposed over MCP. Loran's MCP \
                     surface is read-only by design — `update`, `new`, \
                     `validate`, and `help` are CLI-only (Spec §12.2)."
                ),
            ));
        }

        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let payload = match name {
            "list" => self.tool_list(&args),
            "show" => self.tool_show(&args),
            "find" => self.tool_find(&args),
            "search" => self.tool_search(&args),
            "categories" => self.tool_categories(),
            _ => unreachable!("is_allowed gate already filtered out unknown verbs"),
        }?;

        // MCP wraps tool output as `{ content: [...] }`. We embed the
        // payload as a single `text` block containing the JSON
        // envelope so legacy MCP clients see something sensible, and
        // surface the structured form via the non-standard `data`
        // field that newer Anthropic clients prefer.
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&payload).unwrap_or_default(),
            }],
            "isError": false,
            "data": payload,
        }))
    }

    fn tool_list(&self, args: &Value) -> Result<Value, JsonRpcError> {
        let category = args.get("category").and_then(|v| v.as_str());
        let replaces = args.get("replaces").and_then(|v| v.as_str());
        let safe_alias = args.get("safe_alias_for").and_then(|v| v.as_str());

        let mut pages: Vec<&Page> = self
            .index
            .all()
            .filter(|p| {
                if let Some(cat) = category {
                    if p.category != cat {
                        return false;
                    }
                }
                if let Some(legacy) = replaces {
                    if !p.replaces.iter().any(|r| r == legacy) {
                        return false;
                    }
                }
                if let Some(legacy) = safe_alias {
                    if !p.safe_alias_for.iter().any(|s| s == legacy) {
                        return false;
                    }
                }
                true
            })
            .collect();
        pages.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(serde_json::to_value(pages).unwrap_or(Value::Array(Vec::new())))
    }

    fn tool_show(&self, args: &Value) -> Result<Value, JsonRpcError> {
        let tool = args
            .get("tool")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::new(error_codes::INVALID_PARAMS, "`tool` is required"))?;
        let result = resolve_show(&self.index, tool);
        Ok(serde_json::to_value(result).unwrap_or(Value::Null))
    }

    fn tool_find(&self, args: &Value) -> Result<Value, JsonRpcError> {
        let legacy = args.get("legacy").and_then(|v| v.as_str()).ok_or_else(|| {
            JsonRpcError::new(error_codes::INVALID_PARAMS, "`legacy` is required")
        })?;
        let safe_alias_only = args
            .get("safe_alias_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let result = resolve_find(&self.index, legacy, safe_alias_only);
        Ok(serde_json::to_value(result).unwrap_or(Value::Null))
    }

    fn tool_search(&self, args: &Value) -> Result<Value, JsonRpcError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::new(error_codes::INVALID_PARAMS, "`query` is required"))?;
        let result = resolve_search(&self.index, query);
        Ok(serde_json::to_value(result).unwrap_or(Value::Null))
    }

    fn tool_categories(&self) -> Result<Value, JsonRpcError> {
        let cats = bundled_categories().map_err(|e| {
            JsonRpcError::new(
                error_codes::INTERNAL_ERROR,
                format!("category registry failed to parse: {e}"),
            )
        })?;
        let rows: Vec<Value> = cats
            .iter()
            .map(|(slug, entry)| {
                json!({
                    "name": slug,
                    "title": entry.title,
                    "description": entry.description,
                    "count": self.index.category_count(slug),
                })
            })
            .collect();
        Ok(Value::Array(rows))
    }
}

#[cfg(test)]
mod tests {
    use loran_index::{Index, Ingestor, MarkdownPagesIngestor};
    use serde_json::{Value, json};
    use std::fs;
    use tempfile::TempDir;

    use super::Handler;
    use crate::protocol::{JsonRpcRequest, error_codes};

    fn idx() -> Index {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("file-listing")).unwrap();
        fs::write(
            dir.path().join("file-listing/eza.md"),
            "+++\nname = \"eza\"\ncategory = \"file-listing\"\n\
             summary = \"Modern ls.\"\nreplaces = [\"ls\"]\n+++\nbody\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("file-listing/lsd.md"),
            "+++\nname = \"lsd\"\ncategory = \"file-listing\"\n\
             summary = \"Drop-in ls.\"\nreplaces = [\"ls\"]\n\
             safe_alias_for = [\"ls\"]\n+++\nbody\n",
        )
        .unwrap();
        let pages = MarkdownPagesIngestor::new(dir.path()).ingest().unwrap();
        Index::build(pages).unwrap()
    }

    fn handler() -> Handler {
        Handler::new(idx())
    }

    fn req(method: &str, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!(1)),
            method: method.to_owned(),
            params,
        }
    }

    #[test]
    fn initialize_advertises_tools_capability() {
        let h = handler();
        let resp = h.handle(&req("initialize", json!({}))).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(
            result.pointer("/protocolVersion").and_then(|v| v.as_str()),
            Some("2024-11-05")
        );
        assert!(result.pointer("/capabilities/tools").is_some());
        assert_eq!(
            result.pointer("/serverInfo/name").and_then(|v| v.as_str()),
            Some("loran")
        );
    }

    #[test]
    fn tools_list_returns_exactly_five_read_only_verbs() {
        let h = handler();
        let resp = h.handle(&req("tools/list", json!({}))).unwrap();
        let tools = resp
            .result
            .unwrap()
            .pointer("/tools")
            .cloned()
            .unwrap_or(Value::Null);
        let arr = tools.as_array().expect("tools array");
        let names: Vec<&str> = arr
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert_eq!(names, vec!["list", "show", "find", "search", "categories"]);
    }

    #[test]
    fn tools_call_show_returns_index_hit() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({ "name": "show", "arguments": { "tool": "eza" } }),
            ))
            .unwrap();
        let result = resp.result.expect("show ok");
        let payload = result.pointer("/data").expect("data present");
        assert_eq!(
            payload.pointer("/outcome").and_then(|v| v.as_str()),
            Some("index_hit")
        );
        assert_eq!(
            payload.pointer("/page/name").and_then(|v| v.as_str()),
            Some("eza")
        );
    }

    #[test]
    fn tools_call_show_returns_no_entry_for_unknown_tool() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({ "name": "show", "arguments": { "tool": "definitely-not" } }),
            ))
            .unwrap();
        let payload = resp.result.unwrap().pointer("/data").cloned().unwrap();
        assert_eq!(
            payload.pointer("/outcome").and_then(|v| v.as_str()),
            Some("no_entry")
        );
        assert_eq!(
            payload.pointer("/tool").and_then(|v| v.as_str()),
            Some("definitely-not")
        );
    }

    #[test]
    fn tools_call_find_filters_safe_alias() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({
                    "name": "find",
                    "arguments": { "legacy": "ls", "safe_alias_only": true }
                }),
            ))
            .unwrap();
        let payload = resp.result.unwrap().pointer("/data").cloned().unwrap();
        let names: Vec<&str> = payload
            .pointer("/matches")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"lsd"));
        assert!(!names.contains(&"eza"));
    }

    #[test]
    fn tools_call_search_returns_scored_matches() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({ "name": "search", "arguments": { "query": "eza" } }),
            ))
            .unwrap();
        let payload = resp.result.unwrap().pointer("/data").cloned().unwrap();
        assert!(
            payload
                .pointer("/matches")
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty())
        );
    }

    #[test]
    fn tools_call_list_filters_by_category() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({
                    "name": "list",
                    "arguments": { "category": "file-listing" }
                }),
            ))
            .unwrap();
        let payload = resp.result.unwrap().pointer("/data").cloned().unwrap();
        let names: Vec<&str> = payload
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"eza"));
        assert!(names.contains(&"lsd"));
    }

    #[test]
    fn tools_call_categories_returns_registry_with_counts() {
        let h = handler();
        let resp = h
            .handle(&req("tools/call", json!({ "name": "categories" })))
            .unwrap();
        let payload = resp.result.unwrap().pointer("/data").cloned().unwrap();
        let rows = payload.as_array().unwrap();
        let file_listing = rows
            .iter()
            .find(|r| r.get("name").and_then(|n| n.as_str()) == Some("file-listing"))
            .expect("file-listing row");
        assert!(file_listing.get("count").and_then(Value::as_u64).is_some());
    }

    #[test]
    fn tools_call_write_verb_is_rejected() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({ "name": "update", "arguments": {} }),
            ))
            .unwrap();
        let err = resp.error.expect("rejected");
        assert_eq!(err.code, error_codes::WRITE_VERB_REJECTED);
        assert!(err.message.contains("read-only"));
    }

    #[test]
    fn tools_call_help_verb_is_rejected() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({ "name": "help", "arguments": { "binary": "eza" } }),
            ))
            .unwrap();
        let err = resp.error.expect("rejected");
        assert_eq!(err.code, error_codes::WRITE_VERB_REJECTED);
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let h = handler();
        let resp = h.handle(&req("invalid/method", json!({}))).unwrap();
        let err = resp.error.expect("error");
        assert_eq!(err.code, error_codes::METHOD_NOT_FOUND);
    }

    #[test]
    fn notification_request_returns_none() {
        let h = handler();
        let req_no_id = JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: None,
            method: "initialize".to_owned(),
            params: json!({}),
        };
        assert!(h.handle(&req_no_id).is_none());
    }

    #[test]
    fn show_without_tool_arg_returns_invalid_params() {
        let h = handler();
        let resp = h
            .handle(&req(
                "tools/call",
                json!({ "name": "show", "arguments": {} }),
            ))
            .unwrap();
        let err = resp.error.expect("error");
        assert_eq!(err.code, error_codes::INVALID_PARAMS);
    }

    #[test]
    fn ping_returns_empty_object() {
        let h = handler();
        let resp = h.handle(&req("ping", json!({}))).unwrap();
        assert_eq!(resp.result, Some(json!({})));
    }
}
