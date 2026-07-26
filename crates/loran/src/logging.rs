// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `tracing-subscriber` setup driven by `--verbose` / `--quiet`.
//!
//! Log destination is stderr (stdout is reserved for data per the CLI Standard §6).
//! Quiet → error-only. Default → warn+. `-v` → info+. `-vv` → debug+.
//! `-vvv+` → trace+.

use tracing_subscriber::{EnvFilter, fmt};

use crate::cli::Cli;

/// Install a global `tracing` subscriber.
///
/// `RUST_LOG`, if set, overrides the CLI-flag derivation — that lets
/// developers fine-tune per-crate filtering without rebuilding.
pub(crate) fn init(cli: &Cli) {
    let filter = if let Ok(env_filter) = EnvFilter::try_from_default_env() {
        env_filter
    } else {
        EnvFilter::new(filter_from_flags(cli))
    };

    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .try_init();
}

fn filter_from_flags(cli: &Cli) -> &'static str {
    if cli.global.quiet {
        "error"
    } else {
        match cli.global.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::filter_from_flags;
    use crate::cli::Cli;

    fn parse(args: &[&str]) -> Cli {
        let mut full = vec!["loran"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full).expect("test args parse")
    }

    #[test]
    fn default_filter_is_warn() {
        assert_eq!(filter_from_flags(&parse(&[])), "warn");
    }

    #[test]
    fn quiet_filter_is_error() {
        assert_eq!(filter_from_flags(&parse(&["--quiet"])), "error");
    }

    #[test]
    fn v_filter_is_info() {
        assert_eq!(filter_from_flags(&parse(&["-v"])), "info");
    }

    #[test]
    fn vv_filter_is_debug() {
        assert_eq!(filter_from_flags(&parse(&["-vv"])), "debug");
    }

    #[test]
    fn vvv_or_more_filter_is_trace() {
        assert_eq!(filter_from_flags(&parse(&["-vvv"])), "trace");
        assert_eq!(filter_from_flags(&parse(&["-vvvv"])), "trace");
    }

    #[test]
    fn quiet_beats_verbose() {
        // Both flags allowed; quiet semantics dominate.
        assert_eq!(filter_from_flags(&parse(&["--quiet", "-vv"])), "error");
    }
}
