// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `--version` emission in both human and SFRS-envelope JSON forms.

use std::process::ExitCode;

use jiff::Timestamp;
use serde_json::json;

use crate::cli::{Cli, Format};

const TOOL_NAME: &str = "loran";
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAINTAINER: &str = "Mohamed Hammad <Mohamed.Hammad@Steelbore.com>";
const WEBSITE: &str = "https://Loran.Steelbore.com/";
const SOURCE: &str = "https://github.com/Steelbore/Loran";

/// Print the version banner and return an [`ExitCode`].
///
/// Human form: `tool version` on the first line, then a blank line, then
/// the attribution block. JSON form: the SFRS §6 envelope with
/// `metadata.maintainer` and `metadata.website` populated per Standard
/// §13.2.
pub(crate) fn emit(cli: &Cli) -> ExitCode {
    match cli.output_format() {
        Format::Human => emit_human(),
        Format::Json => emit_json(),
    }
    ExitCode::from(0)
}

fn emit_human() {
    println!("{TOOL_NAME} {TOOL_VERSION}");
    println!();
    println!("Maintained by {MAINTAINER}");
    println!("Project: {WEBSITE}");
    println!("Source: {SOURCE}");
}

fn emit_json() {
    let envelope = json!({
        "metadata": {
            "tool": TOOL_NAME,
            "version": TOOL_VERSION,
            "command": format!("{TOOL_NAME} --version"),
            "timestamp": iso8601_utc(),
            "maintainer": MAINTAINER,
            "website": WEBSITE,
            "source": SOURCE,
        },
        "data": {
            "tool": TOOL_NAME,
            "version": TOOL_VERSION,
            "maintainer": MAINTAINER,
            "website": WEBSITE,
            "source": SOURCE,
        }
    });

    // Pretty-printed so the envelope is human-skimmable from a terminal
    // even in JSON mode. Agents that need compact framing can re-emit
    // with their own serializer.
    let rendered = serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| envelope.to_string());
    println!("{rendered}");
}

/// Current wall-clock UTC formatted ISO 8601 with the `Z` suffix
/// (Steelbore Standard §12.5 — Z suffix is mandatory, never an offset).
fn iso8601_utc() -> String {
    let ts = Timestamp::now();
    // jiff's Display for Timestamp emits `2026-05-12T08:30:00Z` by
    // default — already the right shape.
    ts.to_string()
}

#[cfg(test)]
mod tests {
    use super::{TOOL_VERSION, iso8601_utc};

    #[test]
    fn timestamp_ends_with_z() {
        let ts = iso8601_utc();
        assert!(ts.ends_with('Z'), "timestamp must end with Z: got {ts}");
        assert!(
            !ts.contains('+'),
            "timestamp must not carry an offset: {ts}"
        );
    }

    #[test]
    fn tool_version_matches_cargo_metadata() {
        assert_eq!(TOOL_VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn json_envelope_has_required_metadata_fields() {
        let envelope_str = {
            let envelope = serde_json::json!({
                "metadata": {
                    "tool": super::TOOL_NAME,
                    "version": super::TOOL_VERSION,
                    "command": format!("{} --version", super::TOOL_NAME),
                    "timestamp": iso8601_utc(),
                    "maintainer": super::MAINTAINER,
                    "website": super::WEBSITE,
                    "source": super::SOURCE,
                },
                "data": { "tool": super::TOOL_NAME }
            });
            serde_json::to_string(&envelope).unwrap()
        };
        let parsed: serde_json::Value = serde_json::from_str(&envelope_str).unwrap();
        let metadata = parsed.get("metadata").expect("metadata block");
        for required in &[
            "tool",
            "version",
            "command",
            "timestamp",
            "maintainer",
            "website",
        ] {
            assert!(
                metadata.get(*required).is_some(),
                "metadata.{required} must be present"
            );
        }
    }
}
