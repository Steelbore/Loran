// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Stdio MCP server loop.
//!
//! Reads newline-delimited JSON-RPC 2.0 requests from `reader`, hands
//! each parsed request to a [`Handler`], and writes the response back
//! to `writer`. The loop terminates on EOF (typical MCP client
//! shutdown signal — they close stdin).
//!
//! Errors during reading / writing are surfaced via [`ServerError`].
//! Per-request parse failures emit a JSON-RPC error envelope rather
//! than aborting the loop.

use std::io::{BufRead, Write};

use serde_json::Value;
use thiserror::Error;

use crate::handler::Handler;
use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, error_codes};

/// Lifecycle errors for the stdio loop.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("stdin read failed: {0}")]
    Read(std::io::Error),
    #[error("stdout write failed: {0}")]
    Write(std::io::Error),
}

/// Drive an MCP server against `reader` / `writer` until EOF.
///
/// `reader` is typically `BufReader::new(std::io::stdin().lock())`;
/// `writer` is `std::io::stdout().lock()`. The function is generic
/// over both so tests can drive the loop in-process with `Vec<u8>`
/// buffers.
pub fn serve_stdio<R, W>(handler: &Handler, mut reader: R, mut writer: W) -> Result<(), ServerError>
where
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(ServerError::Read)?;
        if read == 0 {
            // EOF — graceful shutdown.
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => handler.handle(&req),
            Err(err) => Some(JsonRpcResponse::err(
                Value::Null,
                JsonRpcError::new(
                    error_codes::PARSE_ERROR,
                    format!("malformed JSON-RPC: {err}"),
                ),
            )),
        };

        if let Some(resp) = response {
            let rendered = serde_json::to_string(&resp).unwrap_or_else(|_| {
                "{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"internal serialise failure\"}}"
                    .to_owned()
            });
            writeln!(writer, "{rendered}").map_err(ServerError::Write)?;
            writer.flush().map_err(ServerError::Write)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::BufReader;

    use loran_index::{Index, Ingestor, MarkdownPagesIngestor};
    use serde_json::Value;
    use tempfile::TempDir;

    use super::serve_stdio;
    use crate::handler::Handler;

    fn build_index() -> Index {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("file-listing")).unwrap();
        fs::write(
            dir.path().join("file-listing/eza.md"),
            "+++\nname = \"eza\"\ncategory = \"file-listing\"\n\
             summary = \"Modern ls.\"\n+++\nbody\n",
        )
        .unwrap();
        let pages = MarkdownPagesIngestor::new(dir.path()).ingest().unwrap();
        Index::build(pages).unwrap()
    }

    #[test]
    fn handshake_then_call_then_eof() {
        let script = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"show","arguments":{"tool":"eza"}}}"#,
            "\n",
        );
        let reader = BufReader::new(script.as_bytes());
        let mut writer: Vec<u8> = Vec::new();

        let handler = Handler::new(build_index());
        serve_stdio(&handler, reader, &mut writer).expect("loop ok");

        let output = String::from_utf8(writer).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "one response per request: {output}");

        // 1. initialize
        let init: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(
            init.pointer("/result/protocolVersion")
                .and_then(|v| v.as_str()),
            Some("2024-11-05")
        );
        // 2. tools/list
        let listed: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(
            listed
                .pointer("/result/tools")
                .and_then(|v| v.as_array())
                .map(Vec::len),
            Some(5)
        );
        // 3. tools/call show eza
        let show: Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(
            show.pointer("/result/data/page/name")
                .and_then(|v| v.as_str()),
            Some("eza")
        );
        assert_eq!(
            show.pointer("/result/data/outcome")
                .and_then(|v| v.as_str()),
            Some("index_hit")
        );
    }

    #[test]
    fn malformed_request_yields_parse_error() {
        let reader = BufReader::new("not json\n".as_bytes());
        let mut writer: Vec<u8> = Vec::new();
        let handler = Handler::new(build_index());
        serve_stdio(&handler, reader, &mut writer).unwrap();

        let output = String::from_utf8(writer).unwrap();
        let resp: Value = serde_json::from_str(output.lines().next().unwrap()).unwrap();
        assert_eq!(
            resp.pointer("/error/code")
                .and_then(serde_json::Value::as_i64),
            Some(-32700)
        );
    }

    #[test]
    fn empty_lines_are_skipped() {
        let script = "\n\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
        let reader = BufReader::new(script.as_bytes());
        let mut writer: Vec<u8> = Vec::new();
        let handler = Handler::new(build_index());
        serve_stdio(&handler, reader, &mut writer).unwrap();

        let output = String::from_utf8(writer).unwrap();
        assert_eq!(output.lines().count(), 1);
    }

    #[test]
    fn notification_is_silently_swallowed() {
        let script = "{\"jsonrpc\":\"2.0\",\"method\":\"ping\"}\n";
        let reader = BufReader::new(script.as_bytes());
        let mut writer: Vec<u8> = Vec::new();
        let handler = Handler::new(build_index());
        serve_stdio(&handler, reader, &mut writer).unwrap();

        let output = String::from_utf8(writer).unwrap();
        assert!(output.is_empty(), "notifications must not emit a reply");
    }
}
