// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Phase 3 (Bloom) end-to-end integration suite (WP-P3.07).
//!
//! Covers cross-cutting flows that no single WP-specific test file
//! exercises end-to-end:
//!
//! - `DescribeIngestor` against a real subprocess on `$PATH` (a
//!   tempdir-resident shell script masquerading as a Steelbore CLI).
//! - The full layered-index precedence chain — synthesised pages sit
//!   under the bundled catalog, which sits under distro / user
//!   overlays, all the way through `loran show --json`.
//! - `loran schema` round-trips a valid JSON document that names
//!   every public type expected by Phase 3 consumers (MCP, agents,
//!   editor plugins).

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn loran() -> Command {
    let mut cmd = Command::cargo_bin("loran").expect("loran binary built");
    cmd.env("LORAN_DISTRO_OVERRIDE", "generic")
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .env_remove("AI_AGENT")
        .env_remove("AGENT")
        .env_remove("CI")
        .env_remove("CLAUDECODE")
        .env_remove("CURSOR_AGENT")
        .env_remove("GEMINI_CLI")
        .env_remove("LORAN_DESCRIBE_BINARIES");
    cmd
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

/// Create an executable shell script at `<dir>/<name>` whose
/// `describe --json` invocation emits `envelope`. Returns the
/// containing directory so the caller can prepend it to `$PATH`.
#[cfg(unix)]
fn install_fake_describer(dir: &Path, name: &str, envelope: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"describe\" ] && [ \"$2\" = \"--json\" ]; then\n  \
           cat <<'EOF'\n{envelope}\nEOF\n\
           exit 0\n\
         fi\n\
         echo 'unsupported sub-command' >&2\n\
         exit 2\n"
    );
    let path = dir.join(name);
    fs::write(&path, script).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    dir.to_path_buf()
}

/// Compose a new PATH with `extra` prepended to the inherited value.
fn prepend_path(extra: &Path) -> std::ffi::OsString {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let mut joined = std::ffi::OsString::from(extra.as_os_str());
    joined.push(":");
    joined.push(inherited);
    joined
}

const FAKE_ENVELOPE: &str = r#"{
    "metadata": {
        "tool": "ferrocast",
        "version": "0.1.0",
        "website": "https://Ferrocast.Steelbore.com"
    },
    "data": {
        "summary": "Steelbore broadcast packaging tool.",
        "commands": [
            { "name": "pack", "summary": "Pack a broadcast." },
            { "name": "verify", "summary": "Verify a broadcast." }
        ]
    }
}"#;

/// `DescribeIngestor` synthesises a Page from a real subprocess on
/// `$PATH`, and that synthesised page surfaces in `loran show --json`.
#[cfg(unix)]
#[test]
fn describe_ingestor_synthesises_page_from_real_subprocess() {
    let bin_dir = TempDir::new().unwrap();
    install_fake_describer(bin_dir.path(), "ferrocast", FAKE_ENVELOPE);
    let xdg = TempDir::new().unwrap();

    let assert = loran()
        .args(["show", "ferrocast", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DESCRIBE_BINARIES", "ferrocast")
        .env("PATH", prepend_path(bin_dir.path()))
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/name").and_then(|v| v.as_str()),
        Some("ferrocast")
    );
    assert_eq!(
        envelope.pointer("/data/category").and_then(|v| v.as_str()),
        Some("steelbore-cli")
    );
    assert!(
        envelope
            .pointer("/data/summary")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("broadcast")),
        "summary must come from describe payload"
    );
    assert_eq!(
        envelope.pointer("/data/official").and_then(|v| v.as_str()),
        Some("https://Ferrocast.Steelbore.com")
    );
}

/// A user overlay overrides anything `DescribeIngestor` synthesised
/// for the same tool — curated curation always wins over the auto-
/// generated fallback.
#[cfg(unix)]
#[test]
fn user_overlay_overrides_describe_synthesised_page() {
    let bin_dir = TempDir::new().unwrap();
    install_fake_describer(bin_dir.path(), "ferrocast", FAKE_ENVELOPE);
    let xdg = TempDir::new().unwrap();

    // Curated user-overlay page that overrides the summary.
    let overlay = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("steelbore-cli");
    write(
        &overlay,
        "ferrocast.md",
        "+++\nname = \"ferrocast\"\nsummary = \"Curated user-overlay summary.\"\n+++\n",
    );

    let assert = loran()
        .args(["show", "ferrocast", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DESCRIBE_BINARIES", "ferrocast")
        .env("PATH", prepend_path(bin_dir.path()))
        .assert()
        .success();
    let envelope: serde_json::Value =
        serde_json::from_str(&String::from_utf8(assert.get_output().stdout.clone()).unwrap())
            .unwrap();
    assert_eq!(
        envelope.pointer("/data/summary").and_then(|v| v.as_str()),
        Some("Curated user-overlay summary.")
    );
}

/// `loran schema` round-trips a self-consistent Draft 2020-12
/// document that names every type Phase 3 consumers depend on (MCP
/// `tools/list` schema, agent envelope validation).
#[test]
fn schema_document_round_trips_phase3_invariants() {
    let assert = loran().arg("schema").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Draft 2020-12 declaration.
    assert_eq!(
        parsed.get("$schema").and_then(|v| v.as_str()),
        Some("https://json-schema.org/draft/2020-12/schema")
    );

    // Every type Phase 3 needs is in `$defs`.
    for must_have in [
        "Page",
        "ShowResult",
        "FindResult",
        "SearchResult",
        "Categories",
    ] {
        assert!(
            parsed.pointer(&format!("/$defs/{must_have}")).is_some(),
            "$defs/{must_have} missing — MCP `tools/list` cannot resolve its schema"
        );
    }

    // `responses` map ties verbs to their schema.
    for verb in ["show", "find", "search", "categories", "list"] {
        let pointer = parsed
            .pointer(&format!("/responses/{verb}"))
            .expect("responses entry");
        // Each is either a `$ref` to `$defs/...` or a small inline
        // object (the `list` array case).
        assert!(
            pointer.get("$ref").is_some() || pointer.get("type").is_some(),
            "responses/{verb} must be a $ref or inline schema"
        );
    }
}

/// `loran mcp` advertises the five read-only verbs over the real
/// subprocess (handshake → tools/list end-to-end).
#[test]
fn mcp_advertises_read_only_verbs_end_to_end() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(assert_cmd::cargo::cargo_bin("loran"))
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
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(
            (r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#.to_owned() + "\n")
                .as_bytes(),
        )
        .unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let resp: serde_json::Value = serde_json::from_str(
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    let names: Vec<&str> = resp
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert_eq!(names, vec!["list", "show", "find", "search", "categories"]);
}
