// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran update` integration tests.
//!
//! These tests exercise the CLI envelope and exit-code contract
//! hermetically — they override the publisher URLs to point at
//! `127.0.0.1:1` (RST on connect) and isolate every XDG path under a
//! per-test tempdir. No real network egress; no host-filesystem
//! pollution.

use assert_cmd::Command;
use tempfile::tempdir;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

/// `loran update` reports per-source results in `data.results` even
/// when the network is unreachable: both `upstream-pages` and
/// `tldr-pages` surface as error rows, and the process exits with
/// `TARBALL_FETCH_FAILED` (7).
#[test]
fn update_json_envelope_carries_per_source_results_on_fetch_failure() {
    let data_home = tempdir().expect("tempdir");
    let cache_home = tempdir().expect("tempdir");

    let assert = loran()
        .args(["update", "--json"])
        .env("XDG_DATA_HOME", data_home.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LORAN_PAGES_MANIFEST_URL", "http://127.0.0.1:1/pages.json")
        .env("LORAN_PAGES_TARBALL_URL", "http://127.0.0.1:1/pages.tar.gz")
        .env(
            "LORAN_PAGES_SIG_URL",
            "http://127.0.0.1:1/pages.tar.gz.minisig",
        )
        .env("LORAN_TLDR_ARCHIVE_URL", "http://127.0.0.1:1/tldr.zip")
        .assert()
        .failure()
        .code(7);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json emits a parseable envelope");

    assert_eq!(
        envelope
            .pointer("/metadata/command")
            .and_then(|v| v.as_str()),
        Some("loran update")
    );

    let results = envelope
        .pointer("/data/results")
        .and_then(|v| v.as_array())
        .expect("data.results is an array");
    assert_eq!(results.len(), 2, "one row per source");

    let upstream = &results[0];
    assert_eq!(
        upstream.pointer("/source").and_then(|v| v.as_str()),
        Some("upstream-pages")
    );
    assert_eq!(
        upstream.pointer("/status").and_then(|v| v.as_str()),
        Some("error")
    );
    assert_eq!(
        upstream.pointer("/error/code").and_then(|v| v.as_str()),
        Some("TARBALL_FETCH_FAILED")
    );
    assert_eq!(
        upstream
            .pointer("/error/exit_code")
            .and_then(serde_json::Value::as_i64),
        Some(7)
    );

    let tldr = &results[1];
    assert_eq!(
        tldr.pointer("/source").and_then(|v| v.as_str()),
        Some("tldr-pages")
    );
    assert_eq!(
        tldr.pointer("/status").and_then(|v| v.as_str()),
        Some("error")
    );
    assert_eq!(
        tldr.pointer("/error/code").and_then(|v| v.as_str()),
        Some("TARBALL_FETCH_FAILED")
    );

    let ts = envelope
        .pointer("/metadata/timestamp")
        .and_then(|v| v.as_str())
        .expect("timestamp present");
    assert!(ts.ends_with('Z'), "timestamp must end with Z: {ts}");
}

/// Text mode prints one diagnostic per source to stderr and exits with
/// the worst code across sources.
#[test]
fn update_text_mode_emits_per_source_error_lines() {
    let data_home = tempdir().expect("tempdir");
    let cache_home = tempdir().expect("tempdir");

    let assert = loran()
        .arg("update")
        .env("XDG_DATA_HOME", data_home.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LORAN_PAGES_MANIFEST_URL", "http://127.0.0.1:1/pages.json")
        .env("LORAN_PAGES_TARBALL_URL", "http://127.0.0.1:1/pages.tar.gz")
        .env(
            "LORAN_PAGES_SIG_URL",
            "http://127.0.0.1:1/pages.tar.gz.minisig",
        )
        .env("LORAN_TLDR_ARCHIVE_URL", "http://127.0.0.1:1/tldr.zip")
        .assert()
        .failure()
        .code(7);

    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("upstream-pages: error:"),
        "upstream row missing from stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("tldr-pages: error:"),
        "tldr row missing from stderr:\n{stderr}"
    );
}

/// `--dry-run` still goes through the manifest fetch (so the
/// connection-refused failure surfaces the same way), but the envelope
/// must record `dry_run = true` on every row.
#[test]
fn update_dry_run_marks_each_row() {
    let data_home = tempdir().expect("tempdir");
    let cache_home = tempdir().expect("tempdir");

    let assert = loran()
        .args(["--dry-run", "update", "--json"])
        .env("XDG_DATA_HOME", data_home.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LORAN_PAGES_MANIFEST_URL", "http://127.0.0.1:1/pages.json")
        .env("LORAN_PAGES_TARBALL_URL", "http://127.0.0.1:1/pages.tar.gz")
        .env(
            "LORAN_PAGES_SIG_URL",
            "http://127.0.0.1:1/pages.tar.gz.minisig",
        )
        .env("LORAN_TLDR_ARCHIVE_URL", "http://127.0.0.1:1/tldr.zip")
        .assert()
        .failure()
        .code(7);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = envelope
        .pointer("/data/results")
        .and_then(|v| v.as_array())
        .expect("data.results is an array");
    for row in results {
        let dry = row.pointer("/dry_run").and_then(serde_json::Value::as_bool);
        assert_eq!(
            dry,
            Some(true),
            "every row must record dry_run = true, got {row:?}"
        );
    }
}
