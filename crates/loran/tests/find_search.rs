// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran find` + `loran search` integration tests.

use assert_cmd::Command;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

// ─── find ─────────────────────────────────────────────────────────────

#[test]
fn find_broad_mode_returns_replacer() {
    let assert = loran().args(["find", "ls"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("eza\tfile-listing\t"), "got {stdout}");
}

#[test]
fn find_strict_mode_only_returns_alias_safe_replacers() {
    // `bat` is safe_alias_for `cat`; should be returned in strict mode.
    let safe = loran()
        .args(["find", "cat", "--safe-alias"])
        .assert()
        .success();
    let stdout = String::from_utf8(safe.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("bat\t"), "got {stdout}");

    // `rg` replaces `grep` but isn't alias-safe — strict mode should
    // return nothing for `grep`.
    let strict = loran()
        .args(["find", "grep", "--safe-alias"])
        .assert()
        .success();
    let stdout = String::from_utf8(strict.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("no Steelbore tool"),
        "expected no-match diagnostic for grep --safe-alias; got {stdout}"
    );
    assert!(stdout.contains("hint: loran search grep"));
}

#[test]
fn find_empty_result_emits_hint() {
    let assert = loran().args(["find", "no-such-tool"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("no Steelbore tool"));
    assert!(stdout.contains("hint: loran search no-such-tool"));
}

#[test]
fn find_json_envelope_carries_query_and_matches() {
    let assert = loran().args(["find", "cat", "--json"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(
        envelope.pointer("/data/query").and_then(|v| v.as_str()),
        Some("cat")
    );
    assert_eq!(
        envelope
            .pointer("/data/safe_alias_only")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    let matches = envelope
        .pointer("/data/matches")
        .and_then(|m| m.as_array())
        .expect("matches array");
    let names: Vec<&str> = matches
        .iter()
        .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"bat"), "got {names:?}");
}

// ─── search ───────────────────────────────────────────────────────────

#[test]
fn search_finds_pages_by_summary_term() {
    let assert = loran().args(["search", "modern ls"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("eza"),
        "search for 'modern ls' must hit eza: {stdout}"
    );
}

#[test]
fn search_empty_result_emits_hint() {
    let assert = loran()
        .args(["search", "zzz-absolutely-nothing-zzz"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("no matches"));
    assert!(stdout.contains("loran list --json"));
}

#[test]
fn search_json_envelope_carries_scored_matches() {
    let assert = loran()
        .args(["search", "file", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(
        envelope.pointer("/data/query").and_then(|v| v.as_str()),
        Some("file")
    );
    let matches = envelope
        .pointer("/data/matches")
        .and_then(|m| m.as_array())
        .expect("matches array");
    assert!(!matches.is_empty(), "search 'file' must produce matches");

    // Each match has { page, score }; score is a u64.
    for entry in matches {
        assert!(entry.get("page").is_some(), "every match has a page object");
        let score = entry
            .get("score")
            .and_then(serde_json::Value::as_u64)
            .expect("score is u64");
        let _ = score;
    }
}
