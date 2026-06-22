// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Opt-in catalog auto-update integration tests.
//!
//! Hermetic: every XDG path is a per-test tempdir, and the publisher
//! URLs point at `127.0.0.1:1` (RST on connect) so an enabled
//! auto-update attempt fails fast without real network egress. The
//! contract under test is that auto-update is opt-in and that a failed
//! refresh never breaks a read verb.

use assert_cmd::Command;
use tempfile::tempdir;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

/// Point the pages publisher at a dead port so any refresh attempt
/// fails immediately.
fn with_dead_publisher(cmd: &mut Command) {
    cmd.env("LORAN_PAGES_MANIFEST_URL", "http://127.0.0.1:1/pages.json")
        .env("LORAN_PAGES_TARBALL_URL", "http://127.0.0.1:1/pages.tar.gz")
        .env(
            "LORAN_PAGES_SIG_URL",
            "http://127.0.0.1:1/pages.tar.gz.minisig",
        );
}

#[test]
fn autoupdate_failure_is_non_fatal() {
    let data_home = tempdir().unwrap();
    let cache_home = tempdir().unwrap();

    let mut cmd = loran();
    cmd.args(["show", "eza"])
        .env("XDG_DATA_HOME", data_home.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .env("LORAN_AUTO_UPDATE", "1")
        .env("LORAN_AUTO_UPDATE_INTERVAL", "0s");
    with_dead_publisher(&mut cmd);

    // The auto-update attempt hits a dead port and fails silently; the
    // command still resolves `eza` from the bundled catalog.
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Modern ls replacement"));
}

#[test]
fn offline_flag_suppresses_autoupdate() {
    let data_home = tempdir().unwrap();
    let cache_home = tempdir().unwrap();

    let mut cmd = loran();
    cmd.args(["show", "eza", "--offline"])
        .env("XDG_DATA_HOME", data_home.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic")
        .env("LORAN_AUTO_UPDATE", "1")
        .env("LORAN_AUTO_UPDATE_INTERVAL", "0s");
    with_dead_publisher(&mut cmd);

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Modern ls replacement"));
}

#[test]
fn autoupdate_disabled_by_default() {
    let data_home = tempdir().unwrap();
    let cache_home = tempdir().unwrap();

    // No LORAN_AUTO_UPDATE: the read verb must not touch the network at
    // all, so even a dead publisher is irrelevant.
    let mut cmd = loran();
    cmd.args(["show", "eza"])
        .env("XDG_DATA_HOME", data_home.path())
        .env("XDG_CACHE_HOME", cache_home.path())
        .env("LORAN_DISTRO_OVERRIDE", "generic");
    with_dead_publisher(&mut cmd);

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Modern ls replacement"));
}
