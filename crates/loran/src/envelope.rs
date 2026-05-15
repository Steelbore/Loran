// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

// `ErrorEnvelope` + `emit_error` are exercised by tests today and will
// be exercised by every sub-command starting in Sub-phase 1C. Allow
// dead-code at the module level until the bin pulls those paths in.
#![allow(dead_code)]

//! SFRS §6 output envelope — the canonical JSON shape every Loran
//! sub-command emits in `--format json` mode.
//!
//! ```text
//! {
//!   "metadata": { tool, version, command, timestamp, maintainer, website },
//!   "data":     <sub-command-specific payload>
//! }
//! ```
//!
//! Errors use a parallel [`ErrorEnvelope`] shape with `code`,
//! `exit_code`, `message`, `hint`, `timestamp`, `command`, and
//! `docs_url`, written to stderr (per SFRS §1 Rule 8 — stdout is for
//! data only, never diagnostics).
//!
//! Timestamps everywhere are `jiff::Timestamp` values serialised to
//! ISO 8601 UTC with the `Z` suffix (Spacecraft Software Standard §12.5 — no
//! offsets, no local time, no exceptions). The custom serialiser is
//! the single authoritative point for that invariant.

use std::io::{self, Write};

use jiff::Timestamp;
use serde::{Serialize, Serializer};

const TOOL_NAME: &str = "loran";
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAINTAINER: &str = "Mohamed Hammad <Mohamed.Hammad@SpacecraftSoftware.org>";
const WEBSITE: &str = "https://Loran.SpacecraftSoftware.org/";

/// Successful-output envelope.
///
/// `T` is the sub-command-specific payload type. Use [`JsonEmitter`] to
/// write `Envelope<T>` to a sink.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Envelope<T: Serialize> {
    pub metadata: Metadata,
    pub data: T,
}

impl<T: Serialize> Envelope<T> {
    /// Construct an envelope tagged with the current wall-clock
    /// timestamp and the resolved command string.
    pub(crate) fn new(command: impl Into<String>, data: T) -> Self {
        Self {
            metadata: Metadata::for_command(command),
            data,
        }
    }
}

/// SFRS §6 metadata block.
///
/// Every successful Loran output carries this exact shape. The fields
/// are alphabetically `command` / `maintainer` / `timestamp` / `tool` /
/// `version` / `website` and serialise in declaration order.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Metadata {
    pub tool: String,
    pub version: String,
    pub command: String,
    #[serde(serialize_with = "serialize_timestamp")]
    pub timestamp: Timestamp,
    pub maintainer: String,
    pub website: String,
}

impl Metadata {
    /// Build a metadata block tagged "now" for the given command
    /// invocation (e.g. `"loran show eza"`).
    pub(crate) fn for_command(command: impl Into<String>) -> Self {
        Self {
            tool: TOOL_NAME.to_owned(),
            version: TOOL_VERSION.to_owned(),
            command: command.into(),
            timestamp: Timestamp::now(),
            maintainer: MAINTAINER.to_owned(),
            website: WEBSITE.to_owned(),
        }
    }
}

/// SFRS §1 Rule 8 error envelope, written to stderr.
///
/// `code` is a stable string identifier (`"NOT_FOUND"`, `"INDEX_NOT_BUILT"`
/// …); `exit_code` is the numeric process exit code; `hint` is a runnable
/// recovery command per the "tips-thinking" discipline; `docs_url` is the
/// section anchor of the spec that defines the error.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErrorBody {
    pub code: String,
    pub exit_code: i32,
    pub message: String,
    pub hint: String,
    #[serde(serialize_with = "serialize_timestamp")]
    pub timestamp: Timestamp,
    pub command: String,
    pub docs_url: Option<String>,
}

impl ErrorEnvelope {
    /// Construct an error envelope with the current timestamp.
    pub(crate) fn new(
        code: impl Into<String>,
        exit_code: i32,
        message: impl Into<String>,
        hint: impl Into<String>,
        command: impl Into<String>,
        docs_url: Option<String>,
    ) -> Self {
        Self {
            error: ErrorBody {
                code: code.into(),
                exit_code,
                message: message.into(),
                hint: hint.into(),
                timestamp: Timestamp::now(),
                command: command.into(),
                docs_url,
            },
        }
    }
}

/// Centralised JSON emitter — every sub-command writes through one of
/// these so the destination (stdout for data, stderr for errors) and
/// the formatting decision (pretty-printed for terminal-friendliness)
/// stay consistent.
pub(crate) struct JsonEmitter<O: Write, E: Write> {
    stdout: O,
    stderr: E,
}

impl<O: Write, E: Write> JsonEmitter<O, E> {
    pub(crate) fn new(stdout: O, stderr: E) -> Self {
        Self { stdout, stderr }
    }

    /// Write a successful envelope to stdout, pretty-printed.
    pub(crate) fn emit_data<T: Serialize>(&mut self, envelope: &Envelope<T>) -> io::Result<()> {
        let rendered =
            serde_json::to_string_pretty(envelope).map_err(|e| io::Error::other(e.to_string()))?;
        writeln!(self.stdout, "{rendered}")
    }

    /// Write an error envelope to stderr, pretty-printed.
    pub(crate) fn emit_error(&mut self, envelope: &ErrorEnvelope) -> io::Result<()> {
        let rendered =
            serde_json::to_string_pretty(envelope).map_err(|e| io::Error::other(e.to_string()))?;
        writeln!(self.stderr, "{rendered}")
    }
}

impl JsonEmitter<io::Stdout, io::Stderr> {
    /// Convenience constructor for the production case (stdout / stderr).
    pub(crate) fn stdio() -> Self {
        Self::new(io::stdout(), io::stderr())
    }
}

/// Serialise a [`Timestamp`] as ISO 8601 UTC with the `Z` suffix.
///
/// `jiff::Timestamp::Display` already emits the canonical form; this is
/// the single point in the codebase where that contract is asserted via
/// a `serde` `serialize_with`, so the Z-suffix invariant cannot be
/// silently broken by a future refactor.
fn serialize_timestamp<S: Serializer>(ts: &Timestamp, s: S) -> Result<S::Ok, S::Error> {
    let formatted = ts.to_string();
    debug_assert!(
        formatted.ends_with('Z'),
        "jiff::Timestamp must serialise with Z suffix (Standard §12.5)"
    );
    s.serialize_str(&formatted)
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::Value;

    use super::{
        Envelope, ErrorEnvelope, JsonEmitter, MAINTAINER, Metadata, TOOL_NAME, TOOL_VERSION,
        WEBSITE,
    };

    #[derive(Serialize)]
    struct DataPayload {
        name: &'static str,
    }

    #[test]
    fn envelope_round_trips_with_z_suffix_timestamp() {
        let env = Envelope::new("loran show eza", DataPayload { name: "eza" });
        let json = serde_json::to_string(&env).expect("envelope serialises");
        let parsed: Value = serde_json::from_str(&json).expect("envelope is valid JSON");

        let metadata = parsed.get("metadata").expect("metadata block");
        assert_eq!(
            metadata.get("tool").and_then(Value::as_str),
            Some(TOOL_NAME)
        );
        assert_eq!(
            metadata.get("version").and_then(Value::as_str),
            Some(TOOL_VERSION)
        );
        assert_eq!(
            metadata.get("command").and_then(Value::as_str),
            Some("loran show eza")
        );
        assert_eq!(
            metadata.get("maintainer").and_then(Value::as_str),
            Some(MAINTAINER)
        );
        assert_eq!(
            metadata.get("website").and_then(Value::as_str),
            Some(WEBSITE)
        );

        let ts = metadata
            .get("timestamp")
            .and_then(Value::as_str)
            .expect("timestamp present");
        assert!(ts.ends_with('Z'), "timestamp must end with Z: {ts}");
        assert!(!ts.contains('+'), "timestamp must not carry offset: {ts}");

        assert_eq!(
            parsed.pointer("/data/name").and_then(Value::as_str),
            Some("eza")
        );
    }

    #[test]
    fn metadata_for_command_uses_current_clock() {
        let m = Metadata::for_command("loran list");
        // Sanity: the metadata's timestamp is not a default-zero value.
        // jiff::Timestamp::now() never returns the Unix epoch in 2026.
        let formatted = m.timestamp.to_string();
        assert!(formatted.starts_with("20"), "year sane: got {formatted}");
        assert!(formatted.ends_with('Z'));
    }

    #[test]
    fn error_envelope_carries_every_sfrs_required_field() {
        let env = ErrorEnvelope::new(
            "NOT_FOUND",
            3,
            "page `nope` is not in the catalog",
            "loran search nope --json",
            "loran show nope",
            Some("https://Loran.SpacecraftSoftware.org/spec/#9".to_owned()),
        );
        let json = serde_json::to_string(&env).expect("error envelope serialises");
        let parsed: Value = serde_json::from_str(&json).expect("valid JSON");
        let body = parsed.get("error").expect("error block");

        for field in [
            "code",
            "exit_code",
            "message",
            "hint",
            "timestamp",
            "command",
            "docs_url",
        ] {
            assert!(
                body.get(field).is_some(),
                "error.{field} must be present per SFRS §1 Rule 8"
            );
        }

        assert_eq!(body.get("code").and_then(Value::as_str), Some("NOT_FOUND"));
        assert_eq!(body.get("exit_code").and_then(Value::as_i64), Some(3));
        let ts = body
            .get("timestamp")
            .and_then(Value::as_str)
            .expect("timestamp str");
        assert!(ts.ends_with('Z'));
    }

    #[test]
    fn error_envelope_docs_url_can_be_null() {
        let env = ErrorEnvelope::new("X", 1, "m", "h", "loran x", None);
        let json = serde_json::to_string(&env).expect("serialises with None docs_url");
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.pointer("/error/docs_url").unwrap().is_null());
    }

    #[test]
    fn json_emitter_writes_data_to_stdout_sink_and_error_to_stderr_sink() {
        let mut stdout: Vec<u8> = Vec::new();
        let mut stderr: Vec<u8> = Vec::new();
        {
            let mut emitter = JsonEmitter::new(&mut stdout, &mut stderr);
            let data_env = Envelope::new("loran list", DataPayload { name: "eza" });
            emitter.emit_data(&data_env).expect("emit_data succeeds");

            let err_env = ErrorEnvelope::new(
                "NOT_FOUND",
                3,
                "missing",
                "loran search x",
                "loran show x",
                None,
            );
            emitter.emit_error(&err_env).expect("emit_error succeeds");
        }

        let stdout_str = String::from_utf8(stdout).unwrap();
        let stderr_str = String::from_utf8(stderr).unwrap();
        assert!(
            stdout_str.contains("\"data\""),
            "stdout carries the data envelope"
        );
        assert!(
            !stdout_str.contains("\"error\""),
            "stdout must never carry the error envelope (SFRS §1 Rule 8)"
        );
        assert!(
            stderr_str.contains("\"error\""),
            "stderr carries the error envelope"
        );
    }

    #[test]
    fn data_envelope_pretty_printed_with_newline() {
        let mut stdout: Vec<u8> = Vec::new();
        let mut stderr: Vec<u8> = Vec::new();
        {
            let mut emitter = JsonEmitter::new(&mut stdout, &mut stderr);
            let env = Envelope::new("loran list", DataPayload { name: "x" });
            emitter.emit_data(&env).unwrap();
        }
        let stdout_str = String::from_utf8(stdout).unwrap();
        assert!(
            stdout_str.ends_with('\n'),
            "emit_data writes a trailing newline"
        );
        assert!(
            stdout_str.contains('\n'),
            "pretty-printed JSON must contain inner newlines"
        );
    }
}
