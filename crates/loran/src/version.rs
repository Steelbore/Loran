// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `--version` emission in both human and CLI-Standard-envelope JSON forms.

use std::process::ExitCode;

use serde::Serialize;

use crate::cli::{Cli, Format};
use crate::envelope::{Envelope, JsonEmitter};

const TOOL_NAME: &str = "loran";
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAINTAINER: &str = "Mohamed Hammad <Mohamed.Hammad@SpacecraftSoftware.org>";
const WEBSITE: &str = "https://Loran.SpacecraftSoftware.org/";
const SOURCE: &str = "https://github.com/Spacecraft-Software/Loran";

/// Body payload for `--version --json`.
#[derive(Serialize)]
struct VersionData {
    tool: &'static str,
    version: &'static str,
    maintainer: &'static str,
    website: &'static str,
    source: &'static str,
}

/// Print the version banner and return an [`ExitCode`].
///
/// Human form: `tool version` on the first line, blank line, attribution
/// block. JSON form: the CLI Standard's `Envelope<VersionData>` with the metadata
/// fields supplied by [`crate::envelope::Metadata`].
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
    let envelope = Envelope::new(
        format!("{TOOL_NAME} --version"),
        VersionData {
            tool: TOOL_NAME,
            version: TOOL_VERSION,
            maintainer: MAINTAINER,
            website: WEBSITE,
            source: SOURCE,
        },
    );
    let mut emitter = JsonEmitter::stdio();
    let _ = emitter.emit_data(&envelope);
}

#[cfg(test)]
mod tests {
    use super::TOOL_VERSION;

    #[test]
    fn tool_version_matches_cargo_metadata() {
        assert_eq!(TOOL_VERSION, env!("CARGO_PKG_VERSION"));
    }
}
