// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran categories` integration tests.

use assert_cmd::Command;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn categories_text_emits_one_line_per_category_with_count() {
    let assert = loran().arg("categories").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Lines are tab-separated `slug\ttitle\tcount`. The seed
    // registry declares 10 categories.
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 5,
        "expected ≥5 categories in the seed registry; got {lines:?}"
    );
    for line in &lines {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            cols.len(),
            3,
            "format is slug\\ttitle\\tcount; got {cols:?}"
        );
        // Third column must parse as a non-negative integer.
        let count: usize = cols[2].parse().expect("count column is a number");
        let _ = count;
    }

    // Sanity: file-listing must appear with a non-zero count (eza is
    // in the seed catalogue).
    let file_listing = lines
        .iter()
        .find(|l| l.starts_with("file-listing\t"))
        .expect("file-listing row present");
    let cols: Vec<&str> = file_listing.split('\t').collect();
    let count: usize = cols[2].parse().unwrap();
    assert!(count >= 1, "file-listing must have ≥1 page; got {count}");
}

#[test]
fn categories_json_envelope_includes_slug_title_description_count() {
    let assert = loran().args(["categories", "--json"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("envelope is valid JSON");

    let data = envelope
        .pointer("/data")
        .and_then(|d| d.as_array())
        .expect("data is an array");
    assert!(!data.is_empty());

    for entry in data {
        for field in ["name", "title", "description", "count"] {
            assert!(
                entry.get(field).is_some(),
                "every entry must carry `{field}`: {entry:?}"
            );
        }
    }

    let file_listing = data
        .iter()
        .find(|e| e.get("name").and_then(|n| n.as_str()) == Some("file-listing"))
        .expect("file-listing entry present");
    assert_eq!(
        file_listing.get("title").and_then(|v| v.as_str()),
        Some("File listing")
    );
    assert!(
        file_listing
            .get("count")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|c| c >= 1),
        "file-listing count must be ≥1"
    );
}
