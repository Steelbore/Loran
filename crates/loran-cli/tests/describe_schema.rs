// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran describe` + `loran schema` integration tests.

use assert_cmd::Command;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn describe_manifest_lists_every_subcommand_with_capability_tags() {
    let assert = loran().arg("describe").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("describe envelope is JSON");

    assert_eq!(
        envelope.pointer("/data/tool").and_then(|v| v.as_str()),
        Some("loran")
    );
    let commands = envelope
        .pointer("/data/commands")
        .and_then(|c| c.as_array())
        .expect("commands array");

    let names: Vec<&str> = commands
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect();
    for expected in [
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
            names.contains(&expected),
            "describe must list `{expected}`; got {names:?}"
        );
    }

    // Every entry has a non-empty summary and ≥1 capability tag.
    for entry in commands {
        let summary = entry
            .get("summary")
            .and_then(|s| s.as_str())
            .expect("summary present and string");
        assert!(!summary.is_empty(), "summary non-empty: {entry:?}");

        let tags = entry
            .get("capability_tags")
            .and_then(|t| t.as_array())
            .expect("capability_tags array");
        assert!(!tags.is_empty(), "≥1 capability tag: {entry:?}");
    }

    // Global flags + exit codes also present.
    let flags = envelope
        .pointer("/data/global_flags")
        .and_then(|f| f.as_array())
        .expect("global_flags array");
    assert!(flags.len() >= 10, "≥10 global flags surfaced");

    let exit_codes = envelope
        .pointer("/data/exit_codes")
        .and_then(|c| c.as_array())
        .expect("exit_codes array");
    assert_eq!(
        exit_codes.len(),
        12,
        "all 12 exit codes (0..11) surfaced in the manifest"
    );
}

#[test]
fn schema_emits_full_draft_2020_12_document() {
    let assert = loran().arg("schema").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("schema output is valid JSON");

    assert_eq!(
        parsed.get("$schema").and_then(|s| s.as_str()),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "must declare Draft 2020-12"
    );
    assert!(
        parsed.get("$id").and_then(|s| s.as_str()).is_some(),
        "must carry $id"
    );

    // Every public type appears in `$defs`.
    for name in [
        "Page",
        "OverlayPage",
        "IntroBlock",
        "BodyBlock",
        "ShowResult",
        "FindResult",
        "ScoredMatch",
        "SearchResult",
        "CategoryEntry",
        "Categories",
        "UpdateOutcome",
    ] {
        assert!(
            parsed
                .pointer(&format!("/$defs/{name}"))
                .and_then(|v| v.as_object())
                .is_some(),
            "$defs/{name} must be an object schema"
        );
    }

    // Sub-command responses navigate to their schema via $ref.
    for verb in ["show", "find", "search", "categories", "list", "update"] {
        assert!(
            parsed.pointer(&format!("/responses/{verb}")).is_some(),
            "responses/{verb} missing"
        );
    }
}

#[test]
fn schema_with_page_key_emits_just_the_page_subschema() {
    let assert = loran().args(["schema", "page"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(
        parsed.get("$schema").and_then(|s| s.as_str()),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    // The Page sub-schema carries the title schemars generates.
    assert!(
        parsed
            .get("title")
            .and_then(|s| s.as_str())
            .is_some_and(|t| t.contains("Page")),
        "page sub-schema must carry a Page-derived title"
    );
}

#[test]
fn schema_with_unknown_key_returns_usage_error() {
    loran()
        .args(["schema", "no-such-type"])
        .assert()
        .failure()
        .code(2);
}
