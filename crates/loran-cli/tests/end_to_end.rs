// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! End-to-end cross-verb integration tests (WP-P2.18 broadening).
//!
//! These exercise the full pipeline — author with `loran new`,
//! validate with `loran validate`, and read back through `loran show`
//! / `list` / `search`. Every test runs against a per-test tempdir
//! `$XDG_DATA_HOME` so writers don't pollute the host filesystem and
//! readers see exactly the overlay tree the test seeded.
//!
//! `LORAN_DISTRO_OVERRIDE=generic` pins the distro layer name so
//! `/etc/os-release` is irrelevant to the result.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn loran() -> Command {
    let mut cmd = Command::cargo_bin("loran").expect("loran binary built");
    cmd.env("LORAN_DISTRO_OVERRIDE", "generic")
        .env_remove("EDITOR")
        .env_remove("VISUAL")
        .env_remove("AI_AGENT")
        .env_remove("AGENT")
        .env_remove("CI")
        .env_remove("CLAUDECODE")
        .env_remove("CURSOR_AGENT")
        .env_remove("GEMINI_CLI");
    cmd
}

fn xdg() -> TempDir {
    TempDir::new().unwrap()
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

/// `loran new --no-edit` writes a scaffold; `loran show` immediately
/// finds it via the layered index.
#[test]
fn new_then_show_roundtrip() {
    let xdg = xdg();
    loran()
        .args([
            "new",
            "ripgrep",
            "--category",
            "text-search",
            "--summary",
            "Modern grep replacement.",
            "--replaces",
            "grep",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();

    let assert = loran()
        .args(["show", "ripgrep", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/name").and_then(|v| v.as_str()),
        Some("ripgrep")
    );
    assert_eq!(
        envelope.pointer("/data/summary").and_then(|v| v.as_str()),
        Some("Modern grep replacement.")
    );
}

/// Anything `loran new` writes must also pass `loran validate` — the
/// authoring and validation surfaces share the same parse contract.
#[test]
fn new_then_validate_is_clean() {
    let xdg = xdg();
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
        .assert()
        .success();

    loran()
        .arg("validate")
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
}

/// `loran list --json` includes newly-authored overlay pages.
#[test]
fn new_then_list_reflects_overlay() {
    let xdg = xdg();
    loran()
        .args([
            "new",
            "fd",
            "--category",
            "file-search",
            "--summary",
            "Modern find replacement.",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();

    let assert = loran()
        .args(["list", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let pages = envelope
        .pointer("/data")
        .and_then(|v| v.as_array())
        .expect("data array");
    let names: Vec<&str> = pages
        .iter()
        .filter_map(|p| p.pointer("/name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"fd"), "list must include fd: {names:?}");
}

/// `loran search` finds overlay-added tools.
#[test]
fn new_then_search_reflects_overlay() {
    let xdg = xdg();
    loran()
        .args([
            "new",
            "broot",
            "--category",
            "file-listing",
            "--summary",
            "Interactive tree view.",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();

    let assert = loran()
        .args(["search", "broot", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let matches = envelope
        .pointer("/data/matches")
        .and_then(|v| v.as_array())
        .expect("matches array");
    let names: Vec<&str> = matches
        .iter()
        .filter_map(|m| m.pointer("/page/name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        names.contains(&"broot"),
        "search must surface broot: {names:?}"
    );
}

/// A partial overlay in the user layer overrides the bundled
/// summary without disturbing the other fields. Verifies the
/// field-by-field merge end-to-end through `loran show`.
#[test]
fn user_overlay_overrides_bundled_summary_only() {
    let xdg = xdg();
    let overlay = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("file-listing");
    write(
        &overlay,
        "eza.md",
        "+++\nname = \"eza\"\nsummary = \"Local override.\"\n+++\n",
    );

    let assert = loran()
        .args(["show", "eza", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/summary").and_then(|v| v.as_str()),
        Some("Local override.")
    );
    // Category should still be the bundled value.
    assert_eq!(
        envelope.pointer("/data/category").and_then(|v| v.as_str()),
        Some("file-listing")
    );
}

/// The distro overlay (selected via `LORAN_DISTRO_OVERRIDE`) sits
/// between upstream and user. A user-layer override beats it.
#[test]
fn user_overlay_beats_distro_overlay() {
    let xdg = xdg();
    let distro_dir = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("bravais")
        .join("file-listing");
    let user_dir = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("file-listing");
    write(
        &distro_dir,
        "eza.md",
        "+++\nname = \"eza\"\nsummary = \"Bravais override.\"\n+++\n",
    );
    write(
        &user_dir,
        "eza.md",
        "+++\nname = \"eza\"\nsummary = \"User override wins.\"\n+++\n",
    );

    let assert = loran()
        .args(["show", "eza", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "bravais")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/summary").and_then(|v| v.as_str()),
        Some("User override wins.")
    );
}

/// A user overlay can introduce a wholly new tool not present in the
/// bundled catalog; downstream verbs all surface it.
#[test]
fn user_overlay_introduces_new_tool() {
    let xdg = xdg();
    let overlay = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("system-monitoring");
    write(
        &overlay,
        "btop.md",
        "+++\n\
         name = \"btop\"\n\
         category = \"system-monitoring\"\n\
         summary = \"Resource monitor.\"\n\
         +++\n",
    );

    // show
    loran()
        .args(["show", "btop"])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
    // validate
    loran()
        .arg("validate")
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
}

/// Authoring → validation contract: a malformed overlay (handwritten
/// edge case the user might create after scaffolding) fails
/// `loran validate` even though `loran new` wrote a valid file.
#[test]
fn validate_catches_hand_edited_breakage() {
    let xdg = xdg();
    loran()
        .args([
            "new",
            "fd",
            "--category",
            "file-search",
            "--summary",
            "Modern find replacement.",
            "--no-edit",
        ])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();

    // Hand-edit: append an unknown frontmatter key.
    let scaffold = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("file-search")
        .join("fd.md");
    let body = fs::read_to_string(&scaffold).unwrap();
    let broken = body.replace(
        "name           =",
        "name           =\nrepleaces      = []\nname           =",
    );
    fs::write(&scaffold, broken).unwrap();

    loran()
        .arg("validate")
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .failure()
        .code(8);
}

/// `loran categories` counts include overlay-introduced tools.
#[test]
fn categories_count_reflects_overlay_additions() {
    let xdg = xdg();
    let overlay = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("system-monitoring");
    write(
        &overlay,
        "btop.md",
        "+++\n\
         name = \"btop\"\n\
         category = \"system-monitoring\"\n\
         summary = \"Resource monitor.\"\n\
         +++\n",
    );

    let assert = loran()
        .args(["categories", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = envelope
        .pointer("/data")
        .and_then(|v| v.as_array())
        .expect("data array");
    let sysmon = rows
        .iter()
        .find(|r| r.pointer("/name").and_then(|n| n.as_str()) == Some("system-monitoring"))
        .expect("system-monitoring row present");
    let count = sysmon
        .pointer("/count")
        .and_then(serde_json::Value::as_u64)
        .expect("count present");
    assert!(
        count >= 1,
        "system-monitoring count must include the overlay btop: got {count}"
    );
}
