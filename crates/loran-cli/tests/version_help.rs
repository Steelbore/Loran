// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Integration tests asserting the `--version` and `--help` surfaces
//! match Steelbore Standard v1.1 §13.2 attribution requirements.

use assert_cmd::Command;
use predicates::str::contains;

const MAINTAINER_LINE: &str = "Maintained by Mohamed Hammad <Mohamed.Hammad@Steelbore.com>";
const PROJECT_URL_LINE: &str = "Project: https://Loran.Steelbore.com/";

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn version_human_contains_attribution_block() {
    loran()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("loran 0.0.0"))
        .stdout(contains(MAINTAINER_LINE))
        .stdout(contains(PROJECT_URL_LINE));
}

#[test]
fn version_json_envelope_carries_metadata_maintainer_and_website() {
    let assert = loran().args(["--version", "--json"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf-8");

    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("--version --json emits a JSON envelope");
    let metadata = envelope.get("metadata").expect("metadata block");
    assert_eq!(metadata.get("tool").and_then(|v| v.as_str()), Some("loran"));
    assert_eq!(
        metadata.get("maintainer").and_then(|v| v.as_str()),
        Some("Mohamed Hammad <Mohamed.Hammad@Steelbore.com>")
    );
    assert_eq!(
        metadata.get("website").and_then(|v| v.as_str()),
        Some("https://Loran.Steelbore.com/")
    );

    let ts = metadata
        .get("timestamp")
        .and_then(|v| v.as_str())
        .expect("metadata.timestamp present and a string");
    assert!(ts.ends_with('Z'), "timestamp must end with Z: {ts}");
    assert!(!ts.contains('+'), "timestamp must not carry offset: {ts}");
}

#[test]
fn version_via_format_json_is_equivalent_to_version_json() {
    let assert = loran()
        .args(["--version", "--format", "json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf-8");
    let envelope: serde_json::Value = serde_json::from_str(&stdout).expect("envelope parses");
    assert_eq!(
        envelope.pointer("/metadata/tool").and_then(|v| v.as_str()),
        Some("loran")
    );
}

#[test]
fn help_long_form_contains_attribution_footer() {
    loran()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains(MAINTAINER_LINE))
        .stdout(contains(PROJECT_URL_LINE));
}

#[test]
fn help_short_form_does_not_omit_attribution() {
    loran()
        .arg("-h")
        .assert()
        .success()
        .stdout(contains(MAINTAINER_LINE));
}

#[test]
fn every_sub_command_appears_in_help_output() {
    let assert = loran().arg("--help").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for verb in [
        "list",
        "show",
        "help",
        "find",
        "search",
        "categories",
        "new",
        "update",
        "validate",
        "schema",
        "describe",
        "mcp",
    ] {
        assert!(
            stdout.contains(verb),
            "verb `{verb}` must appear in --help output"
        );
    }
}

#[test]
fn global_flag_is_accepted_before_or_after_subcommand() {
    // `list` is implemented; both orderings should succeed.
    loran().args(["--json", "list"]).assert().success();
    loran().args(["list", "--json"]).assert().success();
}

#[test]
fn unknown_subcommand_returns_clap_usage_error_code_two() {
    loran()
        .arg("definitely-not-a-real-verb")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn pager_flag_is_visible_on_help_sub_help() {
    let assert = loran().args(["help", "--help"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("--pager"),
        "loran help --help must surface the --pager flag"
    );
}
