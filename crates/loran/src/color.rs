// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Color-mode resolution per Steelbore SFRS / [NO_COLOR](https://no-color.org).
//!
//! Resolution order (highest priority first):
//!
//! 1. `--no-color` (CLI) → never.
//! 2. `--color=<auto|always|never>` (CLI) → that value.
//! 3. `NO_COLOR` env var present (any value, including empty) → never.
//! 4. `CLICOLOR_FORCE` or `FORCE_COLOR` env var present → always.
//! 5. Auto: enabled iff stdout is a TTY.

use std::env;

use is_terminal::IsTerminal;

use crate::cli::{Cli, ColorMode};

/// Resolved color decision.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ColorDecision {
    Enabled,
    Disabled,
}

/// Resolve the final color decision from CLI + environment.
pub(crate) fn resolve(cli: &Cli) -> ColorDecision {
    if cli.global.no_color {
        return ColorDecision::Disabled;
    }
    if let Some(mode) = cli.global.color {
        return match mode {
            ColorMode::Always => ColorDecision::Enabled,
            ColorMode::Never => ColorDecision::Disabled,
            ColorMode::Auto => auto_decision(),
        };
    }
    if env::var_os("NO_COLOR").is_some() {
        return ColorDecision::Disabled;
    }
    if env::var_os("CLICOLOR_FORCE").is_some() || env::var_os("FORCE_COLOR").is_some() {
        return ColorDecision::Enabled;
    }
    auto_decision()
}

fn auto_decision() -> ColorDecision {
    if std::io::stdout().is_terminal() {
        ColorDecision::Enabled
    } else {
        ColorDecision::Disabled
    }
}

#[cfg(test)]
mod tests {
    use super::ColorDecision;

    #[test]
    fn enabled_and_disabled_are_distinct() {
        assert_ne!(ColorDecision::Enabled, ColorDecision::Disabled);
    }
}
