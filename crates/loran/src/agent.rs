// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

// Detection helpers + OutputMode variants are exercised by tests today
// and will be called from every sub-command starting in Sub-phase 1C.
#![allow(dead_code)]

//! Output-mode resolution: TTY auto-detection plus the CLI Standard §5
//! "agent guard rail" that forces JSON output whenever a known
//! coding-agent runner is observed in the environment.
//!
//! Resolution cascade (highest priority first):
//!
//! 1. Explicit `--json` or `--format json` on the CLI → [`OutputMode::Json`].
//! 2. Any of the agent env vars (`AI_AGENT`, `AGENT`, `CI`,
//!    `CLAUDECODE`, `CURSOR_AGENT`, `GEMINI_CLI`) is present in the
//!    environment with a non-empty value → [`OutputMode::Json`] with a
//!    one-line stderr warning naming the triggering variable.
//! 3. Stdout is not a TTY → [`OutputMode::Json`] (pipe-friendly default).
//! 4. Stdout is a TTY → [`OutputMode::Tui`] in Phase 2+, collapsed to
//!    [`OutputMode::Text`] in Phase 1 until the TUI lands.
//!
//! Environment access is mediated by the [`EnvSource`] trait so tests
//! can drive the cascade without mutating real `std::env` state.

use std::env;
use std::io::Write;

use crate::cli::Cli;

/// Known agent-runner env vars per `spacecraft-agentic-cli` §3. Order
/// matters: the first match wins when multiple variables are set.
const AGENT_ENV_VARS: &[&str] = &[
    "AI_AGENT",
    "AGENT",
    "CI",
    "CLAUDECODE",
    "CURSOR_AGENT",
    "GEMINI_CLI",
];

/// Final output-mode decision.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutputMode {
    /// Curated palette + ratatui surface (Phase 2+). In Phase 1 this
    /// variant is unreachable from [`detect_output_mode`] — present so
    /// future code paths can match on it without churning the enum.
    Tui,
    /// Plain ANSI-free text suitable for piping. The Phase 1 default
    /// when stdout is a TTY.
    Text,
    /// CLI-Standard-shaped JSON envelope, written to stdout.
    Json,
}

/// What triggered the resolved mode. Surfaced in the stderr warning
/// (when applicable) and exposed for tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutputModeReason {
    /// Explicit `--json` or `--format json` flag.
    Flag,
    /// One of the [`AGENT_ENV_VARS`] resolved with a non-empty value.
    AgentEnv(&'static str),
    /// Stdout is not connected to a TTY.
    NotATty,
    /// Stdout is a TTY and no override applies.
    Tty,
}

/// Decision returned by [`detect_output_mode`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutputDecision {
    pub mode: OutputMode,
    pub reason: OutputModeReason,
}

/// Abstraction over environment-variable reads. The production impl
/// is [`SystemEnv`]; tests use a [`MapEnv`] backed by a `HashMap`.
pub(crate) trait EnvSource {
    fn get(&self, key: &str) -> Option<String>;
}

/// Production [`EnvSource`] reading from `std::env`.
#[derive(Default)]
pub(crate) struct SystemEnv;

impl EnvSource for SystemEnv {
    fn get(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

/// Resolve the output mode from CLI flags + environment + TTY state.
///
/// `stdout_is_tty` is a parameter (not auto-detected) so callers can
/// short-circuit detection in tests and so future surface design can
/// reuse this fn against a buffered writer.
pub(crate) fn detect_output_mode(
    cli: &Cli,
    env: &dyn EnvSource,
    stdout_is_tty: bool,
) -> OutputDecision {
    // 1. Explicit flag override
    if cli.global.json || cli.global.format == Some(crate::cli::Format::Json) {
        return OutputDecision {
            mode: OutputMode::Json,
            reason: OutputModeReason::Flag,
        };
    }

    // 2. Agent env-var guard rail
    if let Some(var) = first_set_agent_var(env) {
        return OutputDecision {
            mode: OutputMode::Json,
            reason: OutputModeReason::AgentEnv(var),
        };
    }

    // 3. Pipe → JSON
    if !stdout_is_tty {
        return OutputDecision {
            mode: OutputMode::Json,
            reason: OutputModeReason::NotATty,
        };
    }

    // 4. TTY → Text (Phase 1) / Tui (Phase 2+)
    OutputDecision {
        mode: OutputMode::Text,
        reason: OutputModeReason::Tty,
    }
}

/// Write the CLI Standard §5 stderr warning when an agent env var has forced
/// JSON mode. No-op for any other reason.
pub(crate) fn emit_agent_env_warning<W: Write>(
    decision: &OutputDecision,
    mut writer: W,
) -> std::io::Result<()> {
    if let OutputModeReason::AgentEnv(var) = decision.reason {
        writeln!(
            writer,
            "loran: agent environment detected (${var} set); falling back to \
             --format json. Suppress with --quiet."
        )?;
    }
    Ok(())
}

fn first_set_agent_var(env: &dyn EnvSource) -> Option<&'static str> {
    AGENT_ENV_VARS
        .iter()
        .copied()
        .find(|var| env.get(var).is_some_and(|v| !v.is_empty()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use clap::Parser;

    use super::{
        EnvSource, OutputMode, OutputModeReason, detect_output_mode, emit_agent_env_warning,
    };
    use crate::cli::Cli;

    /// Test [`EnvSource`] backed by a `HashMap`. Production code never
    /// constructs this — env access in tests must not touch real
    /// `std::env` state because the global env is shared across threads.
    #[derive(Default)]
    struct MapEnv(HashMap<String, String>);

    impl MapEnv {
        fn new() -> Self {
            Self::default()
        }
        fn set(mut self, key: &str, val: &str) -> Self {
            self.0.insert(key.to_owned(), val.to_owned());
            self
        }
    }

    impl EnvSource for MapEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
    }

    fn parse(args: &[&str]) -> Cli {
        let mut full = vec!["loran"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full).expect("args parse")
    }

    #[test]
    fn explicit_json_flag_wins_over_tty() {
        let cli = parse(&["--json"]);
        let d = detect_output_mode(&cli, &MapEnv::new(), true);
        assert_eq!(d.mode, OutputMode::Json);
        assert_eq!(d.reason, OutputModeReason::Flag);
    }

    #[test]
    fn explicit_format_json_wins_over_tty() {
        let cli = parse(&["--format", "json"]);
        let d = detect_output_mode(&cli, &MapEnv::new(), true);
        assert_eq!(d.mode, OutputMode::Json);
        assert_eq!(d.reason, OutputModeReason::Flag);
    }

    #[test]
    fn ai_agent_env_var_forces_json_even_on_tty() {
        let cli = parse(&[]);
        let env = MapEnv::new().set("AI_AGENT", "1");
        let d = detect_output_mode(&cli, &env, true);
        assert_eq!(d.mode, OutputMode::Json);
        assert_eq!(d.reason, OutputModeReason::AgentEnv("AI_AGENT"));
    }

    #[test]
    fn claudecode_env_var_forces_json() {
        let cli = parse(&[]);
        let env = MapEnv::new().set("CLAUDECODE", "1");
        let d = detect_output_mode(&cli, &env, true);
        assert_eq!(d.mode, OutputMode::Json);
        assert_eq!(d.reason, OutputModeReason::AgentEnv("CLAUDECODE"));
    }

    #[test]
    fn empty_string_does_not_count_as_set() {
        let cli = parse(&[]);
        let env = MapEnv::new().set("AI_AGENT", "");
        let d = detect_output_mode(&cli, &env, true);
        // Empty value means "not really set" — fall through to TTY branch.
        assert_eq!(d.mode, OutputMode::Text);
        assert_eq!(d.reason, OutputModeReason::Tty);
    }

    #[test]
    fn earliest_agent_var_wins_when_multiple_set() {
        let cli = parse(&[]);
        let env = MapEnv::new().set("CURSOR_AGENT", "1").set("AI_AGENT", "1"); // AI_AGENT is first in the canonical list
        let d = detect_output_mode(&cli, &env, true);
        assert_eq!(d.reason, OutputModeReason::AgentEnv("AI_AGENT"));
    }

    #[test]
    fn non_tty_falls_through_to_json() {
        let cli = parse(&[]);
        let d = detect_output_mode(&cli, &MapEnv::new(), false);
        assert_eq!(d.mode, OutputMode::Json);
        assert_eq!(d.reason, OutputModeReason::NotATty);
    }

    #[test]
    fn tty_with_no_overrides_returns_text_in_phase_1() {
        let cli = parse(&[]);
        let d = detect_output_mode(&cli, &MapEnv::new(), true);
        assert_eq!(d.mode, OutputMode::Text);
        assert_eq!(d.reason, OutputModeReason::Tty);
    }

    #[test]
    fn agent_env_warning_emits_one_line_to_writer() {
        let decision = super::OutputDecision {
            mode: OutputMode::Json,
            reason: OutputModeReason::AgentEnv("AI_AGENT"),
        };
        let mut sink: Vec<u8> = Vec::new();
        emit_agent_env_warning(&decision, &mut sink).unwrap();
        let text = String::from_utf8(sink).unwrap();
        assert!(
            text.contains("AI_AGENT"),
            "warning names the env var: {text}"
        );
        assert!(text.contains("--format json"));
        assert!(text.ends_with('\n'));
        // Single newline, no leading blank line.
        assert_eq!(text.matches('\n').count(), 1);
    }

    #[test]
    fn agent_env_warning_is_noop_for_flag_reason() {
        let decision = super::OutputDecision {
            mode: OutputMode::Json,
            reason: OutputModeReason::Flag,
        };
        let mut sink: Vec<u8> = Vec::new();
        emit_agent_env_warning(&decision, &mut sink).unwrap();
        assert!(sink.is_empty(), "no warning when reason is Flag");
    }

    #[test]
    fn agent_env_warning_is_noop_for_pipe_reason() {
        let decision = super::OutputDecision {
            mode: OutputMode::Json,
            reason: OutputModeReason::NotATty,
        };
        let mut sink: Vec<u8> = Vec::new();
        emit_agent_env_warning(&decision, &mut sink).unwrap();
        assert!(sink.is_empty(), "no warning when reason is NotATty");
    }
}
