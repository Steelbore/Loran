// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran list` integration tests.

use assert_cmd::Command;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn list_text_emits_one_line_per_page_with_three_default_columns() {
    let assert = loran().arg("list").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Each line is tab-separated; default columns are name / category / summary.
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 3,
        "should list multiple pages; got {lines:?}"
    );
    for line in &lines {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            cols.len(),
            3,
            "default columns are name\\tcategory\\tsummary; got {cols:?}"
        );
    }

    // Sanity: eza must appear in the default list.
    assert!(stdout.contains("eza\tfile-listing\t"), "got {stdout}");
}

#[test]
fn list_json_emits_envelope_with_data_array_of_pages() {
    let assert = loran().args(["list", "--json"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("envelope is parseable JSON");

    let data = envelope
        .get("data")
        .expect("data array")
        .as_array()
        .unwrap();
    assert!(!data.is_empty(), "data array must be non-empty");

    // The seed catalog must include eza.
    let names: Vec<&str> = data
        .iter()
        .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        names.contains(&"eza"),
        "names did not include eza: {names:?}"
    );

    // Page::body is serde-skipped — JSON output must not carry the body
    // markdown at the top level (Spec §8 — body lives under `show`'s
    // body block only).
    for entry in data {
        assert!(
            entry.get("body").is_none(),
            "list output must not include the raw body field; got {entry:?}"
        );
    }
}

#[test]
fn list_filter_by_category() {
    let assert = loran()
        .args(["list", "--category=file-listing"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    for line in stdout.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols[1], "file-listing", "every line must be file-listing");
    }
    assert!(stdout.contains("eza"));
}

#[test]
fn list_filter_by_replaces() {
    let assert = loran().args(["list", "--replaces=cat"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    // bat replaces cat; the filter should keep it and drop others.
    assert!(stdout.contains("bat\t"), "got {stdout}");
    assert!(
        !stdout.contains("eza\t"),
        "eza does not replace cat: {stdout}"
    );
}

#[test]
fn list_filter_by_safe_alias_for() {
    let assert = loran()
        .args(["list", "--safe-alias-for=cat"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("bat\t"),
        "bat is safe_alias_for cat: {stdout}"
    );
}

#[test]
fn list_empty_result_when_unknown_category() {
    let assert = loran()
        .args(["list", "--category=no-such-category-zzz"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.trim().is_empty(),
        "expected empty output; got {stdout}"
    );
}

#[test]
fn list_fields_overrides_default_columns_in_text_mode() {
    let assert = loran()
        .args(["list", "--fields=name,replaces"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    for line in stdout.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 2, "--fields=name,replaces yields 2 columns");
    }
    assert!(stdout.contains("eza\tls"));
}
