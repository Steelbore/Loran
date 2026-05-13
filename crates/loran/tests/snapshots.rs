// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `insta`-driven snapshot tests for stable text-mode output across
//! sub-commands. Snapshots normalise volatile bits (timestamps,
//! versions, paths) so they survive across rebuilds.
//!
//! Update snapshots with `cargo insta review` after intentional output
//! changes.

use assert_cmd::Command;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

/// Strip timestamps + version + ephemeral paths from output so
/// snapshots can compare structural shape across runs.
fn normalise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for line in s.lines() {
        let line = scrub_timestamp(line);
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Replace ISO 8601 timestamps (`YYYY-MM-DDTHH:MM:SS…Z`) with `<TS>`.
fn scrub_timestamp(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 19 <= bytes.len()
            && bytes[i].is_ascii_digit()
            && bytes[i + 4] == b'-'
            && bytes[i + 7] == b'-'
            && bytes[i + 10] == b'T'
            && bytes[i + 13] == b':'
            && bytes[i + 16] == b':'
        {
            // Found something that looks like an ISO 8601 prefix.
            // Skip until we hit Z (terminal) or whitespace.
            out.push_str("<TS>");
            i += 19;
            while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\t' && bytes[i] != b'\n' {
                if bytes[i] == b'Z' {
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[test]
fn snapshot_list_default_text_output() {
    let assert = loran().arg("list").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    insta::assert_snapshot!(normalise(&stdout));
}

#[test]
fn snapshot_categories_text_output() {
    let assert = loran().arg("categories").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    insta::assert_snapshot!(normalise(&stdout));
}

#[test]
fn snapshot_find_ls_text_output() {
    let assert = loran().args(["find", "ls"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    insta::assert_snapshot!(normalise(&stdout));
}

#[test]
fn snapshot_show_eza_no_entry_diagnostic() {
    let assert = loran()
        .args(["show", "definitely-not-in-the-catalog-xyz"])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    insta::assert_snapshot!(normalise(&stderr));
}
