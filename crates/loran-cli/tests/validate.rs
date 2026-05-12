// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran validate` integration tests.
//!
//! Tests run with `$XDG_DATA_HOME` pointed at a per-test tempdir so
//! we exercise the real on-disk walk without touching the host
//! filesystem. `LORAN_DISTRO_OVERRIDE` pins the distro overlay name
//! without writing to `/etc/os-release`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn good_page() -> &'static str {
    "+++\n\
     name = \"eza\"\n\
     category = \"file-listing\"\n\
     summary = \"Modern ls replacement.\"\n\
     +++\n\
     \nBody.\n"
}

fn missing_summary() -> &'static str {
    "+++\n\
     name = \"broken\"\n\
     category = \"misc\"\n\
     +++\n"
}

fn long_summary() -> String {
    let summary = "x".repeat(121);
    format!("+++\nname = \"longsum\"\ncategory = \"c\"\nsummary = \"{summary}\"\n+++\n")
}

#[test]
fn validate_succeeds_on_empty_data_dir() {
    let xdg = TempDir::new().unwrap();
    loran()
        .arg("validate")
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success()
        .stdout(predicates::str::contains("0 pages OK"));
}

#[test]
fn validate_succeeds_on_well_formed_upstream_page() {
    let xdg = TempDir::new().unwrap();
    let upstream = xdg.path().join("loran").join("pages");
    write(&upstream, "eza.md", good_page());

    loran()
        .arg("validate")
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success()
        .stdout(predicates::str::contains("1 pages OK"));
}

#[test]
fn validate_accepts_partial_overlay_in_user_root() {
    let xdg = TempDir::new().unwrap();
    let user = xdg.path().join("loran").join("overlays").join("user");
    write(&user, "eza.md", "+++\nname = \"eza\"\n+++\n");

    // No base in upstream — but `loran validate` is a per-file parser
    // check (NOT an index build), so a syntactically-valid partial
    // overlay passes here. The dangling-base check fires at index
    // build time instead.
    loran()
        .arg("validate")
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success();
}

#[test]
fn validate_exits_8_on_missing_required_field() {
    let xdg = TempDir::new().unwrap();
    let upstream = xdg.path().join("loran").join("pages");
    write(&upstream, "broken.md", missing_summary());

    let assert = loran()
        .arg("validate")
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .failure()
        .code(8);

    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("MISSING_FIELD"),
        "stderr missing diagnostic:\n{stderr}"
    );
    assert!(
        stderr.contains("broken.md"),
        "stderr must name the file:\n{stderr}"
    );
}

#[test]
fn validate_json_envelope_reports_per_file_errors() {
    let xdg = TempDir::new().unwrap();
    let upstream = xdg.path().join("loran").join("pages");
    write(&upstream, "ok.md", good_page());
    write(&upstream, "longsum.md", &long_summary());

    let assert = loran()
        .args(["validate", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .failure()
        .code(8);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(
        envelope
            .pointer("/metadata/command")
            .and_then(|v| v.as_str()),
        Some("loran validate")
    );
    assert_eq!(
        envelope
            .pointer("/data/valid")
            .and_then(serde_json::Value::as_u64),
        Some(1)
    );
    assert_eq!(
        envelope
            .pointer("/data/invalid")
            .and_then(serde_json::Value::as_u64),
        Some(1)
    );

    let errors = envelope
        .pointer("/data/errors")
        .and_then(|v| v.as_array())
        .expect("data.errors is an array");
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].pointer("/code").and_then(|v| v.as_str()),
        Some("SUMMARY_TOO_LONG")
    );
    assert_eq!(
        errors[0].pointer("/layer").and_then(|v| v.as_str()),
        Some("upstream")
    );
    let line = errors[0]
        .pointer("/line")
        .and_then(serde_json::Value::as_u64)
        .expect("line present");
    assert!(line >= 1, "line must be 1-based, got {line}");
}

#[test]
fn validate_categorises_invalid_toml_with_line_number() {
    let xdg = TempDir::new().unwrap();
    let upstream = xdg.path().join("loran").join("pages");
    // Malformed TOML — line 4 has the broken assignment.
    write(
        &upstream,
        "bad.md",
        "+++\nname = \"x\"\ncategory = \"c\"\nsummary = this is not valid\n+++\n",
    );

    let assert = loran()
        .args(["validate", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .failure()
        .code(8);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let errors = envelope
        .pointer("/data/errors")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].pointer("/code").and_then(|v| v.as_str()),
        Some("INVALID_TOML")
    );
}
