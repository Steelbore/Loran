// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![deny(unsafe_code)]

//! Loran TUI — `ratatui` application.
//!
//! WP-P2.02 ships the shell: panic-safe terminal initialisation,
//! Spacecraft Software palette, an event loop, a placeholder pane, and the
//! `q` / `Ctrl-C` quit binding. Subsequent WPs layer the browse,
//! detail, and fuzzy-search views on top.
//!
//! Brand boundary: only curated content uses the Spacecraft Software palette;
//! live `--help` capture frames remain monochrome (Spec §2 decision
//! #11). The palette set lives in [`theme`] and never bleeds into
//! the `loran help` rendering path.

mod app;
mod markdown;
mod prompt;
mod terminal;
pub mod theme;

pub use app::{App, run};
pub use prompt::{NewPromptValues, PromptOutcome, run_new_prompt};
pub use terminal::{TerminalGuard, TuiError};
