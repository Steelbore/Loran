// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Panic-safe terminal initialisation + restoration.
//!
//! The TUI puts the terminal into raw mode and switches to an
//! alternate screen. Both states are toxic if the process exits
//! without restoring them: leftover raw mode swallows the user's
//! shell input, leftover alt-screen wipes scrollback. [`TerminalGuard`]
//! wraps the lifecycle in a `Drop` impl, and we also install a panic
//! hook so a `panic!` inside the event loop never leaves the terminal
//! wedged.

use std::io;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use thiserror::Error;

/// TUI lifecycle errors.
#[derive(Debug, Error)]
pub enum TuiError {
    #[error("terminal I/O failure: {0}")]
    Io(#[from] io::Error),
}

/// RAII guard that owns the terminal handle and restores cooked mode
/// on drop. The wrapped [`Terminal<CrosstermBackend<Stdout>>`] is the
/// surface the app draws on.
pub struct TerminalGuard {
    inner: Option<Terminal<CrosstermBackend<io::Stdout>>>,
}

impl std::fmt::Debug for TerminalGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalGuard")
            .field("active", &self.inner.is_some())
            .finish()
    }
}

impl TerminalGuard {
    /// Enter raw mode, switch to the alternate screen, and install a
    /// panic hook that undoes both before the default hook runs.
    pub fn enter() -> Result<Self, TuiError> {
        // Install the panic hook first so a panic during setup still
        // restores whatever state we managed to enter.
        install_panic_hook();

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            inner: Some(terminal),
        })
    }

    /// Borrow the underlying terminal mutably for draw / read calls.
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        self.inner
            .as_mut()
            .expect("terminal is only None after drop")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
        self.inner = None;
    }
}

fn restore_terminal() -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}

/// Wrap the existing panic hook so the terminal is restored before the
/// hook prints the backtrace. Without this, a `panic!` inside the draw
/// loop leaves the user's shell in raw mode + alt-screen.
fn install_panic_hook() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        let original = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = restore_terminal();
            original(info);
        }));
    });
}
