// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran new` integration tests.
//!
//! Each test isolates `$XDG_DATA_HOME` to a per-test tempdir so the
//! scaffold writes don't leak into the host filesystem.

use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn new_writes_scaffolded_page_to_user_overlay() {
    let xdg = TempDir::new().unwrap();
    let assert = loran()
        .args([
            "new",
            "btop",
            "--category",
            "system-monitoring",
            "--summary",
            "Resource monitor.",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("created"), "stdout missing diag: {stdout}");

    let target = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("system-monitoring")
        .join("btop.md");
    let body = fs::read_to_string(&target).expect("scaffold landed on disk");
    assert!(body.starts_with("+++\n"));
    assert!(body.contains("name           = \"btop\""));
    assert!(body.contains("category       = \"system-monitoring\""));
    assert!(body.contains("summary        = \"Resource monitor.\""));
}

#[test]
fn new_writes_replaces_and_safe_alias_lists() {
    let xdg = TempDir::new().unwrap();
    loran()
        .args([
            "new",
            "eza",
            "--category",
            "file-listing",
            "--summary",
            "Modern ls replacement.",
            "--replaces",
            "ls,dir",
            "--safe-alias-for",
            "dir",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .success();

    let body = fs::read_to_string(
        xdg.path()
            .join("loran")
            .join("overlays")
            .join("user")
            .join("file-listing")
            .join("eza.md"),
    )
    .unwrap();
    assert!(body.contains("replaces       = [\"ls\", \"dir\"]"));
    assert!(body.contains("safe_alias_for = [\"dir\"]"));
}

#[test]
fn new_json_envelope_carries_path_and_metadata() {
    let xdg = TempDir::new().unwrap();
    let assert = loran()
        .args([
            "new",
            "fd",
            "--category",
            "file-search",
            "--summary",
            "Find replacement.",
            "--no-edit",
            "--json",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(
        envelope
            .pointer("/metadata/command")
            .and_then(|v| v.as_str()),
        Some("loran new fd")
    );
    assert_eq!(
        envelope.pointer("/data/tool").and_then(|v| v.as_str()),
        Some("fd")
    );
    assert_eq!(
        envelope.pointer("/data/category").and_then(|v| v.as_str()),
        Some("file-search")
    );
    assert_eq!(
        envelope.pointer("/data/scope").and_then(|v| v.as_str()),
        Some("user")
    );
    let path = envelope
        .pointer("/data/path")
        .and_then(|v| v.as_str())
        .expect("path field present");
    assert!(
        path.ends_with("overlays/user/file-search/fd.md"),
        "path should target the user overlay: {path}"
    );
    assert_eq!(
        envelope
            .pointer("/data/edited")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
}

#[test]
fn new_exits_2_when_category_missing() {
    let xdg = TempDir::new().unwrap();
    let assert = loran()
        .args(["new", "btop", "--summary", "x", "--no-edit"])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("--category"));
}

#[test]
fn new_exits_2_when_summary_missing() {
    let xdg = TempDir::new().unwrap();
    let assert = loran()
        .args([
            "new",
            "btop",
            "--category",
            "system-monitoring",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("--summary"));
}

#[test]
fn new_exits_5_when_overlay_file_already_exists() {
    let xdg = TempDir::new().unwrap();
    loran()
        .args([
            "new",
            "btop",
            "--category",
            "system-monitoring",
            "--summary",
            "Resource monitor.",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .success();

    // Second invocation: must not silently overwrite.
    let assert = loran()
        .args([
            "new",
            "btop",
            "--category",
            "system-monitoring",
            "--summary",
            "Resource monitor.",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .failure()
        .code(5);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("already exists"));
}

#[test]
fn new_validates_summary_too_long_before_writing() {
    let xdg = TempDir::new().unwrap();
    let long = "x".repeat(121);
    let assert = loran()
        .args([
            "new",
            "broken",
            "--category",
            "misc",
            "--summary",
            &long,
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .failure()
        .code(8);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("summary"));

    // Nothing should have been written.
    let target = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("misc")
        .join("broken.md");
    assert!(!target.exists(), "no file should land on disk on failure");
}

#[test]
fn new_with_scope_upstream_requires_loran_upstream_path() {
    let xdg = TempDir::new().unwrap();
    let assert = loran()
        .args([
            "new",
            "btop",
            "--category",
            "system-monitoring",
            "--summary",
            "Resource monitor.",
            "--scope",
            "upstream",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("LORAN_UPSTREAM_PATH")
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .failure()
        .code(10);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("LORAN_UPSTREAM_PATH"));
}

#[test]
fn new_with_scope_upstream_writes_under_loran_upstream_path() {
    let xdg = TempDir::new().unwrap();
    let upstream = TempDir::new().unwrap();
    loran()
        .args([
            "new",
            "btop",
            "--category",
            "system-monitoring",
            "--summary",
            "Resource monitor.",
            "--scope",
            "upstream",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_UPSTREAM_PATH", upstream.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .success();

    let target = upstream
        .path()
        .join("pages")
        .join("system-monitoring")
        .join("btop.md");
    assert!(
        target.exists(),
        "upstream-scoped page should land under LORAN_UPSTREAM_PATH/pages/"
    );
}

#[test]
fn new_seeds_user_template_under_data_dir() {
    let xdg = TempDir::new().unwrap();
    loran()
        .args([
            "new",
            "btop",
            "--category",
            "system-monitoring",
            "--summary",
            "Resource monitor.",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .assert()
        .success();

    let template = xdg.path().join("loran").join("templates").join("tool.md");
    assert!(template.exists(), "template should be seeded on first run");
    let body = fs::read_to_string(&template).unwrap();
    assert!(body.contains("{{name}}"));
}
