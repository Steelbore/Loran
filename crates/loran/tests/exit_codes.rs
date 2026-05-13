// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Coverage tests: every exit code reachable through a Phase 1 CLI
//! path fires exactly when documented.
//!
//! Reachable in Phase 1:
//!   - 0 `SUCCESS`              — happy paths (covered elsewhere)
//!   - 1 `GENERAL_ERROR`        — sub-command stub (`mcp`)
//!   - 2 `USAGE_ERROR`          — unknown verb (clap)
//!   - 3 `NOT_FOUND`            — `show <missing>`, `help <missing>`
//!
//! Codes 4-11 are reserved for paths that land in Phase 2+ (permission,
//! conflict, index-not-built, tarball, page-parse, live-help timeout,
//! overlay write, tarball verify). `LiveHelpTimeout` (9) is reachable in
//! principle via `loran help <slow-tool>` but inducing it deterministically
//! requires fixture binaries; covered as a Phase 2 sub-task.

use assert_cmd::Command;

fn loran() -> Command {
    Command::cargo_bin("loran").expect("loran binary built")
}

#[test]
fn exit_0_on_happy_show() {
    loran().args(["show", "eza"]).assert().success().code(0);
}

#[test]
fn exit_0_on_happy_list() {
    loran().arg("list").assert().success().code(0);
}

#[test]
fn exit_0_on_describe() {
    loran().arg("describe").assert().success().code(0);
}

#[test]
fn exit_2_on_unknown_verb() {
    loran()
        .arg("definitely-not-a-real-verb")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn exit_3_on_show_missing_tool() {
    loran()
        .args(["show", "no-such-tool"])
        .assert()
        .failure()
        .code(3);
}

#[test]
fn exit_3_on_help_missing_binary() {
    loran()
        .args(["help", "zzz-binary-does-not-exist"])
        .assert()
        .failure()
        .code(3);
}

#[test]
fn exit_3_on_help_path_traversal() {
    loran()
        .args(["help", "../etc/passwd"])
        .assert()
        .failure()
        .code(3);
}
