// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Active-distro detection for the overlay engine.
//!
//! Spec §5.1 picks the active overlay from `/etc/os-release`. `ID=bravais`
//! and `ID=ferrite` are the supported Spacecraft Software distro identifiers; any
//! other (or no) file falls back to `"generic"`.
//!
//! Parsing is forgiving: we only look at the `ID=` line, ignore
//! surrounding whitespace, and tolerate POSIX-shell quoting around the
//! value (a real os-release file uses unquoted lowercase, but Debian
//! and friends sometimes ship `ID="debian"`).

use std::fs;
use std::path::Path;

/// Canonical fallback identifier when `/etc/os-release` is missing,
/// malformed, or names an unrecognised distro.
pub const DISTRO_GENERIC: &str = "generic";

/// Default `/etc/os-release` path. Exposed so tests in this module and
/// elsewhere can use [`detect_distro_id_from`] against fixtures.
pub const DEFAULT_OS_RELEASE_PATH: &str = "/etc/os-release";

/// Detect the active Spacecraft Software-distro identifier from the host
/// `/etc/os-release`, returning `"generic"` on any failure.
#[must_use]
pub fn detect_distro_id() -> String {
    detect_distro_id_from(Path::new(DEFAULT_OS_RELEASE_PATH))
}

/// Parse a specific os-release file. The returned identifier is always
/// lower-case; unknown IDs are passed through verbatim so a future
/// overlay can add itself without code changes here.
#[must_use]
pub fn detect_distro_id_from(path: &Path) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return DISTRO_GENERIC.to_owned();
    };
    parse_id(&text).unwrap_or_else(|| DISTRO_GENERIC.to_owned())
}

fn parse_id(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("ID=") {
            let value = rest.trim();
            // Strip matched single or double quotes.
            let unquoted = strip_matched_quotes(value);
            if unquoted.is_empty() {
                continue;
            }
            return Some(unquoted.to_lowercase());
        }
    }
    None
}

fn strip_matched_quotes(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &value[1..value.len() - 1];
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{DISTRO_GENERIC, detect_distro_id_from};

    fn write(name: &str, body: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(name), body).unwrap();
        dir
    }

    #[test]
    fn detects_bravais_id() {
        let dir = write("os-release", "NAME=Bravais\nID=bravais\nVERSION=0.1\n");
        let id = detect_distro_id_from(&dir.path().join("os-release"));
        assert_eq!(id, "bravais");
    }

    #[test]
    fn detects_ferrite_id_with_double_quotes() {
        let dir = write("os-release", "ID=\"ferrite\"\nVERSION=0.1\n");
        let id = detect_distro_id_from(&dir.path().join("os-release"));
        assert_eq!(id, "ferrite");
    }

    #[test]
    fn detects_id_with_single_quotes() {
        let dir = write("os-release", "ID='bravais'\n");
        let id = detect_distro_id_from(&dir.path().join("os-release"));
        assert_eq!(id, "bravais");
    }

    #[test]
    fn lowercases_unknown_ids_for_pass_through() {
        let dir = write("os-release", "ID=MyDistro\n");
        let id = detect_distro_id_from(&dir.path().join("os-release"));
        assert_eq!(id, "mydistro");
    }

    #[test]
    fn missing_file_falls_back_to_generic() {
        let dir = tempdir().unwrap();
        let id = detect_distro_id_from(&dir.path().join("does-not-exist"));
        assert_eq!(id, DISTRO_GENERIC);
    }

    #[test]
    fn empty_id_line_falls_back_to_generic() {
        let dir = write("os-release", "ID=\nNAME=foo\n");
        let id = detect_distro_id_from(&dir.path().join("os-release"));
        assert_eq!(id, DISTRO_GENERIC);
    }

    #[test]
    fn no_id_line_falls_back_to_generic() {
        let dir = write("os-release", "NAME=Foo\nVERSION=1\n");
        let id = detect_distro_id_from(&dir.path().join("os-release"));
        assert_eq!(id, DISTRO_GENERIC);
    }

    #[test]
    fn id_line_must_match_exactly_at_line_start() {
        // VARIANT_ID= shouldn't be picked up.
        let dir = write("os-release", "VARIANT_ID=server\nNAME=foo\n");
        let id = detect_distro_id_from(&dir.path().join("os-release"));
        assert_eq!(id, DISTRO_GENERIC);
    }
}
