// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran mcp` integration tests.
//!
//! Spawn the real binary with a scripted stdin containing newline-
//! delimited JSON-RPC requests, capture stdout, and verify the
//! protocol responses end-to-end.

use std::io::Write;
use std::process::{Command, Stdio};

fn loran_path() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin("loran")
}

/// Drive `loran mcp` with `script` piped to stdin, returning stdout.
fn drive_mcp(script: &str) -> String {
    let mut child = Command::new(loran_path())
        .arg("mcp")
        .env_remove("AI_AGENT")
        .env_remove("AGENT")
        .env_remove("CI")
        .env_remove("CLAUDECODE")
        .env_remove("CURSOR_AGENT")
        .env_remove("GEMINI_CLI")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn loran mcp");
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(script.as_bytes())
        .expect("write stdin");
    // Closing stdin (by dropping the handle) signals EOF to the loop.
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait_with_output");
    assert!(
        output.status.success(),
        "loran mcp exited non-zero. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf-8")
}

#[test]
fn mcp_responds_to_initialize() {
    let script = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_owned() + "\n";
    let stdout = drive_mcp(&script);
    let resp: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("one line")).unwrap();
    assert_eq!(
        resp.pointer("/result/protocolVersion")
            .and_then(|v| v.as_str()),
        Some("2024-11-05")
    );
    assert_eq!(
        resp.pointer("/result/serverInfo/name")
            .and_then(|v| v.as_str()),
        Some("loran")
    );
}

#[test]
fn mcp_tools_list_returns_five_read_only_verbs() {
    let script = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#.to_owned() + "\n";
    let stdout = drive_mcp(&script);
    let resp: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let names: Vec<&str> = resp
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .expect("tools array")
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert_eq!(names, vec!["list", "show", "find", "search", "categories"]);
}

#[test]
fn mcp_tools_call_show_returns_real_bundled_page() {
    let script = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"show","arguments":{"tool":"eza"}}}"#.to_owned() + "\n";
    let stdout = drive_mcp(&script);
    let resp: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    assert_eq!(
        resp.pointer("/result/data/outcome")
            .and_then(|v| v.as_str()),
        Some("index_hit"),
    );
    assert_eq!(
        resp.pointer("/result/data/page/name")
            .and_then(|v| v.as_str()),
        Some("eza"),
    );
}

#[test]
fn mcp_rejects_write_verbs() {
    let script = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"update","arguments":{}}}"#.to_owned() + "\n";
    let stdout = drive_mcp(&script);
    let resp: serde_json::Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    let code = resp
        .pointer("/error/code")
        .and_then(serde_json::Value::as_i64)
        .expect("error code");
    assert_eq!(code, -32001, "WRITE_VERB_REJECTED");
}
