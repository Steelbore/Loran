// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran help` integration tests.
//!
//! Uses `echo` as the captured "tool" — it's available on every
//! POSIX path and `echo --help` either prints help text (GNU) or
//! the literal `--help` (BSD). Either way the captured text is
//! non-empty so success-path assertions hold.

use assert_cmd::Command;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn help_text_emits_de_themed_frame_with_live_output_header() {
    let assert = loran().args(["help", "echo"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("LIVE OUTPUT"),
        "frame header missing: {stdout}"
    );
    assert!(
        stdout.contains("uncurated, captured from `echo --help`"),
        "frame must name the tool + flag: {stdout}"
    );
    assert!(
        stdout.contains("[pager:"),
        "frame must close with the pager metadata line: {stdout}"
    );
}

#[test]
fn help_json_envelope_has_live_help_body() {
    let assert = loran().args(["help", "echo", "--json"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("envelope is valid JSON");

    assert_eq!(
        envelope.pointer("/data/tool").and_then(|v| v.as_str()),
        Some("echo")
    );
    assert_eq!(
        envelope.pointer("/data/body/kind").and_then(|v| v.as_str()),
        Some("live_help"),
        "body.kind must be live_help (Spec §4.2)"
    );
    assert!(
        envelope
            .pointer("/data/body/captured_text")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty()),
        "captured_text must be non-empty"
    );
    let captured_at = envelope
        .pointer("/data/body/captured_at")
        .and_then(|v| v.as_str())
        .expect("captured_at present");
    assert!(
        captured_at.ends_with('Z'),
        "captured_at Z-suffix: {captured_at}"
    );

    // pager_command and pager_source surface per the §4.2.1 amendment.
    let pager_command = envelope
        .pointer("/data/body/pager_command")
        .and_then(|v| v.as_str())
        .expect("pager_command present");
    assert!(!pager_command.is_empty(), "pager_command must be non-empty");
    let pager_source = envelope
        .pointer("/data/body/pager_source")
        .and_then(|v| v.as_str())
        .expect("pager_source present");
    assert!(
        ["flag", "manpager-env", "pager-env", "bat", "moor", "cat"].contains(&pager_source),
        "pager_source `{pager_source}` must be one of the documented variants"
    );
}

#[test]
fn help_with_pager_flag_records_flag_source() {
    let assert = loran()
        .args(["help", "echo", "--pager", "less -R", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope
            .pointer("/data/body/pager_source")
            .and_then(|v| v.as_str()),
        Some("flag")
    );
    assert_eq!(
        envelope
            .pointer("/data/body/pager_command")
            .and_then(|v| v.as_str()),
        Some("less -R")
    );
}

#[test]
fn help_with_loran_sentinel_skips_user_env() {
    let assert = loran()
        .args(["help", "echo", "--pager", "loran", "--json"])
        .env("PAGER", "less")
        .env("MANPAGER", "less -RFX")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let source = envelope
        .pointer("/data/body/pager_source")
        .and_then(|v| v.as_str())
        .expect("pager_source present");
    assert!(
        ["bat", "moor", "cat"].contains(&source),
        "`--pager=loran` must force the Steelbore default chain, got `{source}`"
    );
}

#[test]
fn help_with_empty_pager_disables_pagination_as_cat() {
    let assert = loran()
        .args(["help", "echo", "--pager", "", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope
            .pointer("/data/body/pager_source")
            .and_then(|v| v.as_str()),
        Some("cat")
    );
    assert_eq!(
        envelope
            .pointer("/data/body/pager_command")
            .and_then(|v| v.as_str()),
        Some("cat")
    );
}

#[test]
fn help_unknown_binary_returns_not_found() {
    let assert = loran()
        .args(["help", "zzz-no-such-binary-zzz"])
        .assert()
        .failure()
        .code(3);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("not found"), "got {stderr}");
}

#[test]
fn help_path_traversal_rejected() {
    let assert = loran()
        .args(["help", "../etc/passwd"])
        .assert()
        .failure()
        .code(3);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("looks like a path"), "got {stderr}");
}
