// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Interactive `loran new` prompt (WP-P2.15).
//!
//! A small ratatui form that fills in the required scaffold fields
//! when the user invokes `loran new <tool>` on a TTY without
//! `--no-edit` and without supplying every required flag. The form
//! has three text fields (category, summary, replaces); Tab cycles
//! focus, Enter on the last field commits, Esc / Ctrl-C cancels.
//!
//! Category autocomplete is a hint-only mechanism: as the user types,
//! up to five matching categories from `categories.toml` are shown
//! below the field so the user can pick the canonical slug without
//! leaving the prompt. No automatic substitution; the user always
//! types the value.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::TerminalGuard;
use crate::terminal::TuiError;
use crate::theme::Palette;

const TICK_MS: u64 = 60;

/// What the user entered when the form was committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewPromptValues {
    pub category: String,
    pub summary: String,
    pub replaces: Vec<String>,
}

/// Outcome of [`run_new_prompt`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptOutcome {
    /// User confirmed; values are populated.
    Filled(NewPromptValues),
    /// User cancelled (Esc or Ctrl-C). No values; caller must NOT
    /// touch the filesystem.
    Cancelled,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Field {
    Category,
    Summary,
    Replaces,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Self::Category => Self::Summary,
            Self::Summary => Self::Replaces,
            Self::Replaces => Self::Category,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Category => Self::Replaces,
            Self::Summary => Self::Category,
            Self::Replaces => Self::Summary,
        }
    }
}

/// Form state. Public only via [`run_new_prompt`]; tests construct it
/// directly through the in-crate handle.
#[derive(Debug)]
pub(crate) struct NewPrompt {
    tool: String,
    categories: Vec<String>,
    palette: Palette,
    field: Field,
    category: String,
    summary: String,
    replaces: String,
    done: bool,
    cancelled: bool,
}

impl NewPrompt {
    pub(crate) fn new(
        tool: impl Into<String>,
        categories: Vec<String>,
        palette: Palette,
        prefill: NewPromptValues,
    ) -> Self {
        let replaces = prefill.replaces.join(", ");
        Self {
            tool: tool.into(),
            categories,
            palette,
            field: Field::Category,
            category: prefill.category,
            summary: prefill.summary,
            replaces,
            done: false,
            cancelled: false,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if matches!(key.modifiers, KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'd'))
        {
            self.cancelled = true;
            return;
        }
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => self.cancelled = true,
            (KeyCode::Tab, _) => self.field = self.field.next(),
            (KeyCode::BackTab, _) => self.field = self.field.prev(),
            (KeyCode::Enter, _) => {
                if matches!(self.field, Field::Replaces) {
                    self.done = true;
                } else {
                    self.field = self.field.next();
                }
            }
            (KeyCode::Backspace, _) => {
                self.current_field_mut().pop();
            }
            (KeyCode::Char(c), modifiers)
                if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
            {
                self.current_field_mut().push(c);
            }
            _ => {}
        }
    }

    fn current_field_mut(&mut self) -> &mut String {
        match self.field {
            Field::Category => &mut self.category,
            Field::Summary => &mut self.summary,
            Field::Replaces => &mut self.replaces,
        }
    }

    fn category_hints(&self) -> Vec<String> {
        let query = self.category.trim().to_lowercase();
        let mut matches: Vec<&String> = if query.is_empty() {
            self.categories.iter().take(5).collect()
        } else {
            self.categories
                .iter()
                .filter(|c| c.to_lowercase().contains(&query))
                .take(5)
                .collect()
        };
        matches.sort();
        matches.into_iter().cloned().collect()
    }

    fn outcome(&self) -> Option<PromptOutcome> {
        if self.cancelled {
            return Some(PromptOutcome::Cancelled);
        }
        if !self.done {
            return None;
        }
        let category = self.category.trim().to_owned();
        let summary = self.summary.trim().to_owned();
        if category.is_empty() || summary.is_empty() {
            // Stay in the form until both required fields are non-empty.
            return None;
        }
        let replaces: Vec<String> = self
            .replaces
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        Some(PromptOutcome::Filled(NewPromptValues {
            category,
            summary,
            replaces,
        }))
    }

    fn render(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(frame.area(), 70, 60);
        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);
        let muted = Style::default().fg(palette.muted).bg(palette.background);
        let accent = Style::default()
            .fg(palette.accent)
            .bg(palette.background)
            .add_modifier(Modifier::BOLD);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" new page · {} ", self.tool))
            .style(chrome);
        frame.render_widget(block, area);

        let inner = area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        });
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // tool name
                Constraint::Length(3), // category + hint
                Constraint::Length(3), // matches
                Constraint::Length(2), // summary
                Constraint::Length(2), // replaces
                Constraint::Min(0),
                Constraint::Length(1), // footer
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("tool      ", muted),
                Span::styled(self.tool.clone(), chrome),
            ]))
            .style(chrome),
            chunks[0],
        );

        self.draw_field(frame, chunks[1], Field::Category, "category  ");

        let hints = self.category_hints();
        let hint_line = if hints.is_empty() {
            Line::from(Span::styled(
                "(no matching category in registry — using as-is)".to_owned(),
                muted,
            ))
        } else {
            Line::from(vec![
                Span::styled("matches   ", muted),
                Span::styled(hints.join(", "), accent),
            ])
        };
        frame.render_widget(Paragraph::new(hint_line).style(chrome), chunks[2]);

        self.draw_field(frame, chunks[3], Field::Summary, "summary   ");
        self.draw_field(frame, chunks[4], Field::Replaces, "replaces  ");

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Tab", accent),
                Span::styled(" next  ·  ", muted),
                Span::styled("Shift-Tab", accent),
                Span::styled(" back  ·  ", muted),
                Span::styled("Enter", accent),
                Span::styled(" commit (on replaces)  ·  ", muted),
                Span::styled("Esc", accent),
                Span::styled(" cancel", muted),
            ]))
            .alignment(Alignment::Center)
            .style(chrome),
            chunks[6],
        );
    }

    fn draw_field(&self, frame: &mut Frame<'_>, area: Rect, field: Field, label: &str) {
        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);
        let muted = Style::default().fg(palette.muted).bg(palette.background);
        let accent = Style::default()
            .fg(palette.accent)
            .bg(palette.background)
            .add_modifier(Modifier::BOLD);

        let value = match field {
            Field::Category => &self.category,
            Field::Summary => &self.summary,
            Field::Replaces => &self.replaces,
        };
        let focused = field == self.field;
        let label_style = if focused { accent } else { muted };
        let value_style = chrome;
        let cursor = if focused {
            Span::styled("█".to_owned(), accent)
        } else {
            Span::raw("")
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(label.to_owned(), label_style),
                Span::styled(value.clone(), value_style),
                cursor,
            ]))
            .style(chrome),
            area,
        );
    }
}

/// Centre a rectangle of `pct_x` × `pct_y` percent inside `outer`.
fn centered_rect(outer: Rect, pct_x: u16, pct_y: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(outer);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vertical[1])[1]
}

/// Drive the interactive form to completion. Acquires the terminal,
/// runs the event loop, and restores cooked mode before returning.
pub fn run_new_prompt(
    tool: impl Into<String>,
    categories: Vec<String>,
    palette: Palette,
    prefill: NewPromptValues,
) -> Result<PromptOutcome, TuiError> {
    let mut prompt = NewPrompt::new(tool, categories, palette, prefill);
    let mut guard = TerminalGuard::enter()?;
    let tick = Duration::from_millis(TICK_MS);

    loop {
        guard.terminal().draw(|frame| prompt.render(frame))?;
        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                prompt.handle_key(key);
            }
        }
        if let Some(outcome) = prompt.outcome() {
            return Ok(outcome);
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{Field, NewPrompt, NewPromptValues, PromptOutcome};
    use crate::theme::Palette;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn prompt() -> NewPrompt {
        NewPrompt::new(
            "btop",
            vec![
                "file-listing".to_owned(),
                "process-management".to_owned(),
                "system-monitoring".to_owned(),
            ],
            Palette::monochrome(),
            NewPromptValues {
                category: String::new(),
                summary: String::new(),
                replaces: Vec::new(),
            },
        )
    }

    #[test]
    fn fresh_prompt_focuses_category() {
        let p = prompt();
        assert_eq!(p.field, Field::Category);
        assert!(p.outcome().is_none());
    }

    #[test]
    fn typing_fills_focused_field() {
        let mut p = prompt();
        p.handle_key(key(KeyCode::Char('s')));
        p.handle_key(key(KeyCode::Char('y')));
        p.handle_key(key(KeyCode::Char('s')));
        assert_eq!(p.category, "sys");
    }

    #[test]
    fn tab_advances_focus() {
        let mut p = prompt();
        p.handle_key(key(KeyCode::Tab));
        assert_eq!(p.field, Field::Summary);
        p.handle_key(key(KeyCode::Tab));
        assert_eq!(p.field, Field::Replaces);
        p.handle_key(key(KeyCode::Tab));
        assert_eq!(p.field, Field::Category);
    }

    #[test]
    fn shift_tab_reverses_focus() {
        let mut p = prompt();
        p.handle_key(key(KeyCode::BackTab));
        assert_eq!(p.field, Field::Replaces);
    }

    #[test]
    fn enter_on_intermediate_field_advances() {
        let mut p = prompt();
        p.handle_key(key(KeyCode::Enter));
        assert_eq!(p.field, Field::Summary);
        assert!(p.outcome().is_none());
    }

    #[test]
    fn enter_on_replaces_commits_form() {
        let mut p = prompt();
        for c in "system-monitoring".chars() {
            p.handle_key(key(KeyCode::Char(c)));
        }
        p.handle_key(key(KeyCode::Tab));
        for c in "Resource monitor.".chars() {
            p.handle_key(key(KeyCode::Char(c)));
        }
        p.handle_key(key(KeyCode::Tab));
        for c in "top, htop".chars() {
            p.handle_key(key(KeyCode::Char(c)));
        }
        p.handle_key(key(KeyCode::Enter));

        match p.outcome() {
            Some(PromptOutcome::Filled(v)) => {
                assert_eq!(v.category, "system-monitoring");
                assert_eq!(v.summary, "Resource monitor.");
                assert_eq!(v.replaces, vec!["top", "htop"]);
            }
            other => panic!("expected Filled, got {other:?}"),
        }
    }

    #[test]
    fn esc_cancels() {
        let mut p = prompt();
        p.handle_key(key(KeyCode::Esc));
        assert_eq!(p.outcome(), Some(PromptOutcome::Cancelled));
    }

    #[test]
    fn ctrl_c_cancels() {
        let mut p = prompt();
        p.handle_key(ctrl(KeyCode::Char('c')));
        assert_eq!(p.outcome(), Some(PromptOutcome::Cancelled));
    }

    #[test]
    fn commit_blocked_when_required_field_empty() {
        let mut p = prompt();
        p.handle_key(key(KeyCode::Tab)); // category empty, but field=Summary
        p.handle_key(key(KeyCode::Tab)); // field=Replaces
        p.handle_key(key(KeyCode::Enter));
        // No outcome yet: category + summary still empty.
        assert!(p.outcome().is_none());
        // Cancel still works.
        p.handle_key(key(KeyCode::Esc));
        assert_eq!(p.outcome(), Some(PromptOutcome::Cancelled));
    }

    #[test]
    fn category_hints_filter_by_substring() {
        let mut p = prompt();
        for c in "system".chars() {
            p.handle_key(key(KeyCode::Char(c)));
        }
        let hints = p.category_hints();
        assert!(hints.contains(&"system-monitoring".to_owned()));
        assert!(!hints.contains(&"file-listing".to_owned()));
    }

    #[test]
    fn backspace_pops_focused_field() {
        let mut p = prompt();
        p.handle_key(key(KeyCode::Char('a')));
        p.handle_key(key(KeyCode::Char('b')));
        p.handle_key(key(KeyCode::Backspace));
        assert_eq!(p.category, "a");
    }

    #[test]
    fn prefill_seeds_fields() {
        let p = NewPrompt::new(
            "eza",
            vec!["file-listing".to_owned()],
            Palette::monochrome(),
            NewPromptValues {
                category: "file-listing".to_owned(),
                summary: "Modern ls.".to_owned(),
                replaces: vec!["ls".to_owned(), "dir".to_owned()],
            },
        );
        assert_eq!(p.category, "file-listing");
        assert_eq!(p.summary, "Modern ls.");
        assert_eq!(p.replaces, "ls, dir");
    }
}
