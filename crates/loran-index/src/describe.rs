// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `DescribeIngestor` — synthesize Loran pages from SFRS-compliant
//! `<tool> describe --json` output (WP-P3.04).
//!
//! Steelbore CLIs are required to implement a `describe` sub-command
//! that emits an SFRS envelope describing every verb, global flag,
//! and exit code (`steelbore-cli-standard` §7). This ingester walks a
//! caller-supplied list of binaries, runs `describe --json` against
//! each with a 5-second timeout, parses the envelope, and synthesises
//! a baseline [`Page`] per tool. Curated pages overlay on top via the
//! [`LayeredIngestor`](crate::LayeredIngestor) so hand-written
//! Steelbore notes always win.
//!
//! The [`Runner`] trait abstracts subprocess execution so unit tests
//! can drive the parser and synthesiser without spawning real
//! processes.

use std::fmt::Write as _;
use std::process::{Command, Stdio};
use std::time::Duration;

use loran_pages::Page;
use serde::Deserialize;
use thiserror::Error;
use wait_timeout::ChildExt as _;

use crate::error::IngestError;
use crate::ingestor::Ingestor;

/// How long any individual `describe --json` invocation is allowed to
/// take. Conservatively chosen — describe is read-only and should
/// return in <100ms; anything beyond five seconds is almost certainly
/// a hung subprocess and we'd rather the index build proceeds without
/// the offender than block forever.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Canonical category slug for auto-synthesised pages. The bundled
/// `categories.toml` may not declare it — that's fine: `loran
/// categories` only enumerates entries it knows about and gracefully
/// reports `0` counts for any unknown slug. A future Steelbore release
/// can promote this to a registry entry once the convention settles.
pub const SYNTH_CATEGORY: &str = "steelbore-cli";

/// Errors specific to the describe pipeline. They flow up as
/// [`IngestError::BadSource`] so the surrounding pipeline doesn't
/// need a new top-level variant.
#[derive(Debug, Error)]
pub enum DescribeError {
    #[error("`{binary}` describe invocation failed: {source}")]
    Spawn {
        binary: String,
        #[source]
        source: std::io::Error,
    },
    #[error("`{binary} describe --json` timed out after {timeout:?}")]
    Timeout { binary: String, timeout: Duration },
    #[error("`{binary} describe --json` returned non-zero status {status}")]
    NonZeroStatus { binary: String, status: i32 },
    #[error("`{binary}` describe stdout was not valid UTF-8")]
    NotUtf8 { binary: String },
    #[error("`{binary}` describe envelope is not valid JSON: {source}")]
    InvalidJson {
        binary: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("`{binary}` describe envelope is missing required field `{field}`")]
    MissingField { binary: String, field: &'static str },
}

/// Subprocess-execution seam. Production uses [`RealRunner`]; tests
/// build canned implementations that return fixed `stdout` payloads
/// without touching the OS.
pub trait Runner: std::fmt::Debug {
    /// Run `<binary> describe --json` and return its stdout, or a
    /// typed error explaining why we couldn't.
    fn run_describe(&self, binary: &str) -> Result<String, DescribeError>;
}

/// Real subprocess runner. Resolves the binary on `$PATH`, captures
/// stdout, enforces a per-call timeout.
#[derive(Debug, Clone)]
pub struct RealRunner {
    pub timeout: Duration,
}

impl Default for RealRunner {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl Runner for RealRunner {
    fn run_describe(&self, binary: &str) -> Result<String, DescribeError> {
        // Reject path-like names early so a caller can't smuggle in
        // `./malicious-binary`. `which` already enforces this but we
        // want a precise error before paying the spawn cost.
        if binary.contains('/') || binary.contains('\\') || binary.is_empty() {
            return Err(DescribeError::Spawn {
                binary: binary.to_owned(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "binary name must be a bare $PATH lookup",
                ),
            });
        }
        let resolved = which::which(binary).map_err(|err| DescribeError::Spawn {
            binary: binary.to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, err.to_string()),
        })?;

        let mut child = Command::new(&resolved)
            .args(["describe", "--json"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| DescribeError::Spawn {
                binary: binary.to_owned(),
                source,
            })?;

        let status = child
            .wait_timeout(self.timeout)
            .map_err(|source| DescribeError::Spawn {
                binary: binary.to_owned(),
                source,
            })?;

        let Some(status) = status else {
            // Timed out — kill the child and report.
            let _ = child.kill();
            let _ = child.wait();
            return Err(DescribeError::Timeout {
                binary: binary.to_owned(),
                timeout: self.timeout,
            });
        };

        if !status.success() {
            return Err(DescribeError::NonZeroStatus {
                binary: binary.to_owned(),
                status: status.code().unwrap_or(-1),
            });
        }

        // Read stdout. Note: we already have the exit status, so the
        // child has finished — read_to_end can't block.
        let mut buf = Vec::new();
        if let Some(mut stdout) = child.stdout {
            std::io::Read::read_to_end(&mut stdout, &mut buf).map_err(|source| {
                DescribeError::Spawn {
                    binary: binary.to_owned(),
                    source,
                }
            })?;
        }
        String::from_utf8(buf).map_err(|_| DescribeError::NotUtf8 {
            binary: binary.to_owned(),
        })
    }
}

/// SFRS envelope subset we need — only the fields that feed the
/// synthesised Page. `serde(default)` is permissive so a binary that
/// emits an unrelated envelope still produces a parse error at the
/// schema level rather than a hard deserialise crash.
#[derive(Debug, Clone, Deserialize)]
struct DescribeEnvelope {
    metadata: DescribeMetadata,
    data: DescribePayload,
}

#[derive(Debug, Clone, Deserialize)]
struct DescribeMetadata {
    tool: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    website: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DescribePayload {
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    commands: Vec<DescribeCommand>,
}

#[derive(Debug, Clone, Deserialize)]
struct DescribeCommand {
    name: String,
    #[serde(default)]
    summary: Option<String>,
}

/// Parse a `<tool> describe --json` envelope into the typed shape.
fn parse_envelope(binary: &str, raw: &str) -> Result<DescribeEnvelope, DescribeError> {
    let envelope: DescribeEnvelope =
        serde_json::from_str(raw).map_err(|source| DescribeError::InvalidJson {
            binary: binary.to_owned(),
            source,
        })?;
    if envelope.metadata.tool.is_empty() {
        return Err(DescribeError::MissingField {
            binary: binary.to_owned(),
            field: "metadata.tool",
        });
    }
    Ok(envelope)
}

/// Convert a parsed envelope into a baseline Page. The body is a
/// Markdown rendering of the verb list so the user always sees
/// something readable before they author a curated page.
fn synthesise_page(envelope: DescribeEnvelope) -> Result<Page, DescribeError> {
    let tool_name = envelope.metadata.tool.clone();
    let summary = envelope
        .data
        .summary
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Steelbore CLI: {tool_name}."));

    let mut body = String::new();
    body.push_str("## Auto-synthesised from `describe --json`\n\n");
    body.push_str(
        "This page is the default fallback Loran generates by invoking the \
         binary's own `describe` sub-command. Override it with a curated \
         entry under `overlays/user/` to replace this with hand-written \
         Steelbore notes.\n\n",
    );
    if let Some(version) = envelope.metadata.version.as_deref() {
        let _ = writeln!(body, "**Version:** `{version}`\n");
    }
    if let Some(website) = envelope.metadata.website.as_deref() {
        let _ = writeln!(body, "**Website:** <{website}>\n");
    }

    if !envelope.data.commands.is_empty() {
        body.push_str("## Sub-commands\n\n");
        for cmd in &envelope.data.commands {
            let summary_str = cmd.summary.as_deref().unwrap_or("");
            let _ = writeln!(body, "- **{}** — {}", cmd.name, summary_str);
        }
        body.push('\n');
    }

    // Construct a minimal frontmatter and parse it via Page::parse so
    // we hit the same validation gate every other ingestor goes
    // through. This also guarantees a future schema change in Page is
    // automatically applied here.
    let frontmatter = format!(
        "+++\n\
         name = \"{tool}\"\n\
         category = \"{SYNTH_CATEGORY}\"\n\
         summary = \"{summary}\"\n\
         {website_line}\
         +++\n",
        tool = tool_escape(&tool_name),
        summary = toml_escape(&truncate_summary(&summary)),
        website_line = envelope
            .metadata
            .website
            .as_deref()
            .map(|w| format!("official = \"{}\"\n", toml_escape(w)))
            .unwrap_or_default(),
    );
    let mut text = frontmatter;
    text.push('\n');
    text.push_str(&body);

    Page::parse(&text).map_err(|err| DescribeError::InvalidJson {
        binary: envelope.metadata.tool,
        source: serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err.to_string(),
        )),
    })
}

/// `Page::summary` is capped at 120 characters. Auto-synthesised
/// strings can exceed that, so trim with an ellipsis when needed.
fn truncate_summary(s: &str) -> String {
    const CAP: usize = 120;
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= CAP {
        return s.to_owned();
    }
    let mut out: String = chars.iter().take(CAP - 1).collect();
    out.push('…');
    out
}

fn toml_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ")
}

fn tool_escape(value: &str) -> String {
    // Tool names are kebab-case slugs already; defensively reject
    // characters that would break the frontmatter.
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(*c, '-' | '_'))
        .collect()
}

/// Ingester that synthesises baseline pages from `describe --json`
/// output across a caller-supplied list of binaries.
#[derive(Debug)]
pub struct DescribeIngestor {
    binaries: Vec<String>,
    runner: Box<dyn Runner>,
}

impl DescribeIngestor {
    /// Build an ingester over `binaries`, executing each through
    /// `runner`. Use [`Self::with_real_runner`] for production.
    #[must_use]
    pub fn new(binaries: Vec<String>, runner: Box<dyn Runner>) -> Self {
        Self { binaries, runner }
    }

    /// Convenience constructor that pairs `binaries` with the default
    /// [`RealRunner`].
    #[must_use]
    pub fn with_real_runner(binaries: Vec<String>) -> Self {
        Self::new(binaries, Box::new(RealRunner::default()))
    }

    /// Build an ingester from the `LORAN_DESCRIBE_BINARIES`
    /// environment variable, comma-separated. Returns `None` when the
    /// variable is unset or empty so the caller can short-circuit
    /// without spawning anything.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("LORAN_DESCRIBE_BINARIES").ok()?;
        let binaries: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        if binaries.is_empty() {
            return None;
        }
        Some(Self::with_real_runner(binaries))
    }

    /// Inspect the binaries this ingester is configured to walk.
    #[must_use]
    pub fn binaries(&self) -> &[String] {
        &self.binaries
    }
}

impl Ingestor for DescribeIngestor {
    /// Walk every configured binary. A single binary's failure
    /// degrades to a tracing diagnostic and is otherwise skipped so a
    /// missing tool doesn't break the catalog build for the rest.
    fn ingest(&self) -> Result<Vec<Page>, IngestError> {
        let mut pages: Vec<Page> = Vec::with_capacity(self.binaries.len());
        for binary in &self.binaries {
            match self.run_one(binary) {
                Ok(page) => pages.push(page),
                Err(err) => {
                    // Soft failure — log to stderr so an interactive
                    // user sees what went wrong, but keep going.
                    eprintln!("loran: describe ingest skipped `{binary}`: {err}");
                }
            }
        }
        Ok(pages)
    }
}

impl DescribeIngestor {
    fn run_one(&self, binary: &str) -> Result<Page, DescribeError> {
        let raw = self.runner.run_describe(binary)?;
        let envelope = parse_envelope(binary, &raw)?;
        synthesise_page(envelope)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        DescribeError, DescribeIngestor, Runner, SYNTH_CATEGORY, parse_envelope, synthesise_page,
    };
    use crate::Ingestor;

    #[derive(Debug, Default)]
    struct CannedRunner {
        responses: HashMap<String, Result<String, String>>,
    }

    impl CannedRunner {
        fn ok(binary: &str, json: &str) -> Self {
            let mut s = Self::default();
            s.responses.insert(binary.to_owned(), Ok(json.to_owned()));
            s
        }
    }

    impl Runner for CannedRunner {
        fn run_describe(&self, binary: &str) -> Result<String, DescribeError> {
            match self.responses.get(binary) {
                Some(Ok(out)) => Ok(out.clone()),
                Some(Err(_)) => Err(DescribeError::NonZeroStatus {
                    binary: binary.to_owned(),
                    status: 1,
                }),
                None => Err(DescribeError::Spawn {
                    binary: binary.to_owned(),
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "no canned response"),
                }),
            }
        }
    }

    const SAMPLE_ENVELOPE: &str = r#"{
        "metadata": {
            "tool": "ferrocast",
            "version": "0.1.0",
            "website": "https://Ferrocast.Steelbore.com"
        },
        "data": {
            "summary": "Steelbore broadcast packaging tool.",
            "commands": [
                { "name": "pack", "summary": "Pack a broadcast." },
                { "name": "verify", "summary": "Verify a broadcast." }
            ]
        }
    }"#;

    #[test]
    fn parse_envelope_round_trips_metadata_and_commands() {
        let env = parse_envelope("ferrocast", SAMPLE_ENVELOPE).expect("parse ok");
        assert_eq!(env.metadata.tool, "ferrocast");
        assert_eq!(env.data.commands.len(), 2);
    }

    #[test]
    fn parse_envelope_rejects_missing_tool() {
        let raw = r#"{"metadata":{"tool":""},"data":{}}"#;
        let err = parse_envelope("x", raw).unwrap_err();
        assert!(matches!(err, DescribeError::MissingField { .. }));
    }

    #[test]
    fn parse_envelope_rejects_invalid_json() {
        let err = parse_envelope("x", "not json").unwrap_err();
        assert!(matches!(err, DescribeError::InvalidJson { .. }));
    }

    #[test]
    fn synthesise_page_produces_valid_page_with_synth_category() {
        let env = parse_envelope("ferrocast", SAMPLE_ENVELOPE).unwrap();
        let page = synthesise_page(env).expect("page synth ok");
        assert_eq!(page.name, "ferrocast");
        assert_eq!(page.category, SYNTH_CATEGORY);
        assert!(page.summary.starts_with("Steelbore broadcast"));
        assert!(page.body.contains("Auto-synthesised"));
        assert!(page.body.contains("pack"));
        assert!(page.body.contains("verify"));
        assert_eq!(
            page.official.as_deref(),
            Some("https://Ferrocast.Steelbore.com")
        );
    }

    #[test]
    fn synthesise_page_falls_back_to_default_summary_when_missing() {
        let raw = r#"{"metadata":{"tool":"caliper"},"data":{}}"#;
        let env = parse_envelope("caliper", raw).unwrap();
        let page = synthesise_page(env).unwrap();
        assert!(page.summary.contains("caliper"));
    }

    #[test]
    fn synthesise_page_truncates_long_summary() {
        let long = "x".repeat(200);
        let raw = format!(r#"{{"metadata":{{"tool":"x"}},"data":{{"summary":"{long}"}}}}"#);
        let env = parse_envelope("x", &raw).unwrap();
        let page = synthesise_page(env).unwrap();
        assert!(page.summary.chars().count() <= 120);
    }

    #[test]
    fn ingestor_collects_pages_for_each_responsive_binary() {
        let runner = CannedRunner::ok("ferrocast", SAMPLE_ENVELOPE);
        let ingestor = DescribeIngestor::new(vec!["ferrocast".to_owned()], Box::new(runner));
        let pages = ingestor.ingest().unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].name, "ferrocast");
    }

    #[test]
    fn ingestor_skips_failed_binaries_without_aborting() {
        let runner = CannedRunner::ok("ferrocast", SAMPLE_ENVELOPE);
        // `caliper` has no canned response → CannedRunner returns Spawn err.
        let ingestor = DescribeIngestor::new(
            vec!["ferrocast".to_owned(), "caliper".to_owned()],
            Box::new(runner),
        );
        let pages = ingestor.ingest().unwrap();
        assert_eq!(pages.len(), 1, "caliper failure must not abort the walk");
        assert_eq!(pages[0].name, "ferrocast");
    }

    #[test]
    fn from_env_returns_none_when_var_unset() {
        // The env var is virtually never present during cargo test;
        // if a user explicitly exports it the negative test simply
        // doesn't apply and we tolerate that silently rather than
        // mutating shared process env (which Rust 2024 makes unsafe).
        if std::env::var("LORAN_DESCRIBE_BINARIES").is_ok() {
            return;
        }
        assert!(DescribeIngestor::from_env().is_none());
    }

    #[test]
    fn tool_name_with_path_separator_is_rejected_by_real_runner() {
        let runner = super::RealRunner::default();
        for bad in ["", "./ferrocast", "../escape", "abc/def"] {
            assert!(runner.run_describe(bad).is_err(), "should reject `{bad}`");
        }
    }
}
