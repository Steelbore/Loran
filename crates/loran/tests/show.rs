// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `loran show <tool>` integration tests.

use assert_cmd::Command;
use predicates::str::contains;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn show_hit_text_mode_renders_intro_and_body() {
    loran()
        .arg("show")
        .arg("eza")
        .assert()
        .success()
        .stdout(contains("EZA"))
        .stdout(contains("Modern ls replacement"))
        .stdout(contains("From Spacecraft Software curation"))
        .stdout(contains("SPACECRAFT SOFTWARE NOTES"));
}

#[test]
fn show_hit_json_envelope_has_intro_and_body_blocks() {
    let assert = loran().args(["show", "eza", "--json"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json emits a parseable envelope");

    assert_eq!(
        envelope
            .pointer("/metadata/command")
            .and_then(|v| v.as_str()),
        Some("loran show eza")
    );
    assert_eq!(
        envelope.pointer("/data/name").and_then(|v| v.as_str()),
        Some("eza")
    );
    assert_eq!(
        envelope.pointer("/data/category").and_then(|v| v.as_str()),
        Some("file-listing")
    );
    assert_eq!(
        envelope
            .pointer("/data/intro/source")
            .and_then(|v| v.as_str()),
        Some("spacecraft software")
    );
    assert_eq!(
        envelope.pointer("/data/body/kind").and_then(|v| v.as_str()),
        Some("custom")
    );
    assert!(
        envelope
            .pointer("/data/body/body_md")
            .and_then(|v| v.as_str())
            .is_some_and(|b| b.contains("Spacecraft Software notes")),
        "body.body_md should contain the page body"
    );

    // Top-level data must NOT include the raw `body` markdown string —
    // body lives under `data.body.body_md` only (Spec §8). Page::body
    // is serde-skipped to enforce this.
    assert!(
        envelope
            .pointer("/data/body")
            .is_some_and(serde_json::Value::is_object),
        "data.body must be an object, not a markdown string"
    );
}

#[test]
fn show_surfaces_user_overlay_override() {
    let xdg = tempfile::TempDir::new().unwrap();
    let user_overlay = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("file-listing");
    std::fs::create_dir_all(&user_overlay).unwrap();
    std::fs::write(
        user_overlay.join("eza.md"),
        "+++\nname = \"eza\"\nsummary = \"User-pinned summary.\"\n+++\n",
    )
    .unwrap();

    let assert = loran()
        .args(["show", "eza", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/summary").and_then(|v| v.as_str()),
        Some("User-pinned summary."),
        "user overlay must override bundled summary"
    );
    // Category survives — the overlay didn't touch it.
    assert_eq!(
        envelope.pointer("/data/category").and_then(|v| v.as_str()),
        Some("file-listing")
    );
}

#[test]
fn show_surfaces_user_overlay_introduced_tool() {
    let xdg = tempfile::TempDir::new().unwrap();
    let user_overlay = xdg
        .path()
        .join("loran")
        .join("overlays")
        .join("user")
        .join("system-monitoring");
    std::fs::create_dir_all(&user_overlay).unwrap();
    std::fs::write(
        user_overlay.join("btop.md"),
        "+++\n\
         name = \"btop\"\n\
         category = \"system-monitoring\"\n\
         summary = \"Resource monitor.\"\n\
         +++\n",
    )
    .unwrap();

    let assert = loran()
        .args(["show", "btop", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/name").and_then(|v| v.as_str()),
        Some("btop"),
        "user overlay must introduce wholly new pages"
    );
}

#[test]
fn show_miss_text_mode_emits_canonical_no_entry_diagnostic() {
    let assert = loran()
        .args(["show", "definitely-not-in-the-catalog"])
        .assert()
        .failure()
        .code(3);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("no Loran entry for 'definitely-not-in-the-catalog'"));
    assert!(stderr.contains("loran new definitely-not-in-the-catalog --edit"));
    assert!(stderr.contains("loran search"));
    assert!(stderr.contains("loran help"));
}

#[test]
fn show_miss_json_mode_emits_error_envelope_with_hint() {
    let assert = loran()
        .args(["show", "no-such-tool", "--json"])
        .assert()
        .failure()
        .code(3);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(&stderr).expect("error envelope is valid JSON on stderr");

    assert_eq!(
        envelope.pointer("/error/code").and_then(|v| v.as_str()),
        Some("NOT_FOUND")
    );
    assert_eq!(
        envelope
            .pointer("/error/exit_code")
            .and_then(serde_json::Value::as_i64),
        Some(3)
    );
    assert_eq!(
        envelope.pointer("/error/hint").and_then(|v| v.as_str()),
        Some("loran new no-such-tool --edit")
    );
    let ts = envelope
        .pointer("/error/timestamp")
        .and_then(|v| v.as_str())
        .expect("timestamp present");
    assert!(ts.ends_with('Z'), "timestamp must end with Z: {ts}");
}

/// Closed-loop proof: a tool present only in the downloaded upstream
/// tree (`$XDG_DATA_HOME/loran/pages/`, where `loran update` extracts
/// the verified tarball) must surface from `loran show`. Before the
/// `index_loader` fix this directory was written but never read.
#[test]
fn show_surfaces_downloaded_upstream_introduced_tool() {
    let xdg = tempfile::TempDir::new().unwrap();
    let upstream = xdg
        .path()
        .join("loran")
        .join("pages")
        .join("data-processing");
    std::fs::create_dir_all(&upstream).unwrap();
    std::fs::write(
        upstream.join("dasel.md"),
        "+++\n\
         name = \"dasel\"\n\
         category = \"data-processing\"\n\
         summary = \"Query and modify data structures from the shell.\"\n\
         +++\n",
    )
    .unwrap();

    let assert = loran()
        .args(["show", "dasel", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/name").and_then(|v| v.as_str()),
        Some("dasel"),
        "downloaded upstream pages must surface in the read index"
    );
}

/// Precedence chain per Spec §5.1: bundled < downloaded upstream <
/// user overlay. A downloaded page overrides the compiled-in bundled
/// one, and the user overlay overrides the downloaded one in turn.
#[test]
fn show_precedence_bundled_below_downloaded_below_user_overlay() {
    let xdg = tempfile::TempDir::new().unwrap();
    let loran_root = xdg.path().join("loran");

    // Downloaded upstream override of the bundled `eza` page.
    let upstream = loran_root.join("pages").join("file-listing");
    std::fs::create_dir_all(&upstream).unwrap();
    std::fs::write(
        upstream.join("eza.md"),
        "+++\n\
         name = \"eza\"\n\
         category = \"file-listing\"\n\
         summary = \"Downloaded upstream summary.\"\n\
         +++\n",
    )
    .unwrap();

    // Downloaded upstream beats bundled.
    let assert = loran()
        .args(["show", "eza", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/summary").and_then(|v| v.as_str()),
        Some("Downloaded upstream summary."),
        "downloaded upstream must override bundled"
    );

    // Now add a user overlay; it must win over the downloaded page.
    let user_overlay = loran_root
        .join("overlays")
        .join("user")
        .join("file-listing");
    std::fs::create_dir_all(&user_overlay).unwrap();
    std::fs::write(
        user_overlay.join("eza.md"),
        "+++\nname = \"eza\"\nsummary = \"User-pinned summary.\"\n+++\n",
    )
    .unwrap();

    let assert = loran()
        .args(["show", "eza", "--json"])
        .env("XDG_DATA_HOME", xdg.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        envelope.pointer("/data/summary").and_then(|v| v.as_str()),
        Some("User-pinned summary."),
        "user overlay must override the downloaded upstream page"
    );
}
