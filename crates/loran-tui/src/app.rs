// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! TUI application loop.
//!
//! WP-P2.02 ships the shell only: a single placeholder pane that
//! shows the catalog page count and quits on `q`, `Esc`, or `Ctrl-C`.
//! Browse, detail, fuzzy-search, and the in-app help overlay land
//! in subsequent WPs (P2.03 – P2.06).

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use loran_index::Index;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::TerminalGuard;
use crate::terminal::TuiError;
use crate::theme::Palette;

/// Tick frequency for the event loop. 60 ms is enough for "feels
/// instant" key responsiveness without burning CPU on an idle screen.
const TICK_MS: u64 = 60;

/// Top-level TUI application state.
#[derive(Debug)]
pub struct App {
    index: Index,
    palette: Palette,
    should_quit: bool,
}

impl App {
    /// Construct an app over `index` with `palette`. The caller picks
    /// the palette so `NO_COLOR` resolution can happen alongside the
    /// CLI's `--no-color` cascade before the TUI starts.
    #[must_use]
    pub fn new(index: Index, palette: Palette) -> Self {
        Self {
            index,
            palette,
            should_quit: false,
        }
    }

    /// Convenience constructor that pulls the palette from the env.
    #[must_use]
    pub fn from_env(index: Index) -> Self {
        Self::new(index, Palette::from_env())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q') | KeyCode::Esc, _)
            | (KeyCode::Char('c' | 'd'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame<'_>) {
        let area = frame.area();
        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);
        let muted = Style::default().fg(palette.muted).bg(palette.background);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        // Header banner.
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                "LORAN",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("reference manual for Steelbore tools", muted),
        ]))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::BOTTOM).style(chrome));
        frame.render_widget(header, chunks[0]);

        // Placeholder body.
        let body_lines = vec![
            Line::from(Span::styled(
                format!("{} curated pages indexed", self.index.len()),
                chrome,
            )),
            Line::raw(""),
            Line::from(Span::styled("Browse view lands in WP-P2.03", muted)),
            Line::from(Span::styled(
                "Detail view in WP-P2.04 · Search in WP-P2.05",
                muted,
            )),
        ];
        let body = Paragraph::new(body_lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" preview ")
                    .style(chrome),
            );
        frame.render_widget(body, chunks[1]);

        // Footer keybinding hint.
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("q", chrome.add_modifier(Modifier::BOLD)),
            Span::styled(" quit  ·  ", muted),
            Span::styled("Ctrl-C", chrome.add_modifier(Modifier::BOLD)),
            Span::styled(" force quit", muted),
        ]))
        .alignment(Alignment::Center)
        .style(chrome);
        frame.render_widget(footer, chunks[2]);
    }
}

/// Initialise the terminal, run the event loop, and restore on exit.
///
/// Returns when the user quits, after `Drop` on the [`TerminalGuard`]
/// has restored cooked mode + the main screen.
pub fn run(app: &mut App) -> Result<(), TuiError> {
    let mut guard = TerminalGuard::enter()?;
    let tick = Duration::from_millis(TICK_MS);
    while !app.should_quit {
        guard.terminal().draw(|frame| app.render(frame))?;
        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use loran_index::{Index, Ingestor, MarkdownPagesIngestor};
    use tempfile::TempDir;

    use super::App;
    use crate::theme::Palette;

    fn empty_index() -> Index {
        let dir = TempDir::new().unwrap();
        let pages = MarkdownPagesIngestor::new(dir.path()).ingest().unwrap();
        Index::build(pages).unwrap()
    }

    #[test]
    fn q_sets_quit_flag() {
        let mut app = App::new(empty_index(), Palette::monochrome());
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(app.should_quit);
    }

    #[test]
    fn esc_sets_quit_flag() {
        let mut app = App::new(empty_index(), Palette::monochrome());
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_sets_quit_flag() {
        let mut app = App::new(empty_index(), Palette::monochrome());
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_d_sets_quit_flag() {
        let mut app = App::new(empty_index(), Palette::monochrome());
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn unrelated_key_does_not_quit() {
        let mut app = App::new(empty_index(), Palette::monochrome());
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(!app.should_quit);
    }
}
