// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! JSON-RPC 2.0 wire types for the MCP surface.
//!
//! MCP rides newline-delimited JSON-RPC 2.0 on stdio. The wire shape
//! is the standard JSON-RPC envelope: an `id` (string / integer /
//! null), a `method` string, optional `params` object, and the
//! mirroring response carrying either `result` or `error`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 protocol-level error codes (from the spec) plus a
/// small set of MCP-specific extensions.
pub mod error_codes {
    /// Invalid JSON received by the server.
    pub const PARSE_ERROR: i32 = -32700;
    /// JSON is not a valid request object.
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method does not exist.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal server error.
    pub const INTERNAL_ERROR: i32 = -32603;
    /// MCP-specific: write verb requested via the read-only surface.
    pub const WRITE_VERB_REJECTED: i32 = -32001;
}

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    /// `None` for notifications. Lifted to `Value` so string + int IDs
    /// both round-trip without coercion.
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Build a successful response paired with the request id.
    #[must_use]
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Build an error response paired with the request id.
    #[must_use]
    pub fn err(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Constructor for the common case (no `data` payload).
    #[must_use]
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonRpcRequest, JsonRpcResponse, error_codes};
    use serde_json::{Value, json};

    #[test]
    fn request_parses_with_numeric_id() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id.as_ref().and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn request_parses_with_string_id() {
        let raw = r#"{"jsonrpc":"2.0","id":"abc","method":"x"}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.id.as_ref().and_then(Value::as_str), Some("abc"));
    }

    #[test]
    fn notification_has_no_id() {
        let raw = r#"{"jsonrpc":"2.0","method":"notify"}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert!(req.id.is_none());
    }

    #[test]
    fn ok_response_omits_error_field() {
        let resp = JsonRpcResponse::ok(json!(1), json!({"ok": true}));
        let rendered = serde_json::to_string(&resp).unwrap();
        assert!(rendered.contains("\"result\""));
        assert!(!rendered.contains("\"error\""));
    }

    #[test]
    fn err_response_omits_result_field() {
        let err = super::JsonRpcError::new(error_codes::METHOD_NOT_FOUND, "no");
        let resp = JsonRpcResponse::err(json!(2), err);
        let rendered = serde_json::to_string(&resp).unwrap();
        assert!(rendered.contains("\"error\""));
        assert!(!rendered.contains("\"result\""));
    }
}
