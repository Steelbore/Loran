// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! TUI application loop.
//!
//! WP-P2.02 stood up the shell. WP-P2.03 adds a dual-pane browse
//! view: categories on the left, tools-in-selected-category on the
//! right. Tab toggles focus; `j` / `k` and the arrow keys move the
//! selection within the focused pane. Enter records a tool selection
//! that the detail view (WP-P2.04) will consume; `/` and `?` are
//! reserved for the search overlay (P2.05) and in-app help (P2.06).

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use loran_index::Index;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::TerminalGuard;
use crate::terminal::TuiError;
use crate::theme::Palette;

/// Tick frequency for the event loop. 60 ms is enough for "feels
/// instant" key responsiveness without burning CPU on an idle screen.
const TICK_MS: u64 = 60;

/// Which pane currently owns keyboard focus.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    Categories,
    Tools,
}

impl Focus {
    fn toggle(self) -> Self {
        match self {
            Self::Categories => Self::Tools,
            Self::Tools => Self::Categories,
        }
    }
}

/// Top-level TUI application state.
#[derive(Debug)]
pub struct App {
    index: Index,
    palette: Palette,
    categories: Vec<String>,
    focus: Focus,
    cat_state: ListState,
    tool_state: ListState,
    selected_tool: Option<String>,
    should_quit: bool,
}

impl App {
    /// Construct an app over `index` with `palette`.
    #[must_use]
    pub fn new(index: Index, palette: Palette) -> Self {
        let categories: Vec<String> = index.categories().map(str::to_owned).collect();
        let mut cat_state = ListState::default();
        if !categories.is_empty() {
            cat_state.select(Some(0));
        }
        let mut tool_state = ListState::default();
        tool_state.select(None);

        let mut app = Self {
            index,
            palette,
            categories,
            focus: Focus::Categories,
            cat_state,
            tool_state,
            selected_tool: None,
            should_quit: false,
        };
        app.sync_tool_selection();
        app
    }

    /// Convenience constructor that pulls the palette from the env.
    #[must_use]
    pub fn from_env(index: Index) -> Self {
        Self::new(index, Palette::from_env())
    }

    /// Tool names in the currently-selected category, sorted by name.
    fn tools_in_selected_category(&self) -> Vec<String> {
        let Some(cat) = self.selected_category() else {
            return Vec::new();
        };
        let mut tools: Vec<String> = self
            .index
            .by_category(cat)
            .map(|page| page.name.clone())
            .collect();
        tools.sort();
        tools
    }

    fn selected_category(&self) -> Option<&str> {
        self.cat_state
            .selected()
            .and_then(|idx| self.categories.get(idx))
            .map(String::as_str)
    }

    /// Re-anchor `tool_state` after a category change so the right
    /// pane never points at a stale index.
    fn sync_tool_selection(&mut self) {
        let tools = self.tools_in_selected_category();
        if tools.is_empty() {
            self.tool_state.select(None);
        } else {
            let new_idx = self
                .tool_state
                .selected()
                .map_or(0, |i| i.min(tools.len() - 1));
            self.tool_state.select(Some(new_idx));
        }
    }

    fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Categories => Self::nudge(&mut self.cat_state, self.categories.len(), delta),
            Focus::Tools => {
                let len = self.tools_in_selected_category().len();
                Self::nudge(&mut self.tool_state, len, delta);
            }
        }
        if matches!(self.focus, Focus::Categories) {
            self.sync_tool_selection();
        }
    }

    fn nudge(state: &mut ListState, len: usize, delta: isize) {
        if len == 0 {
            state.select(None);
            return;
        }
        let current = state.selected().unwrap_or(0);
        let next = match delta.signum() {
            1 => current.saturating_add(delta.unsigned_abs()).min(len - 1),
            -1 => current.saturating_sub(delta.unsigned_abs()),
            _ => current,
        };
        state.select(Some(next));
    }

    fn confirm(&mut self) {
        if !matches!(self.focus, Focus::Tools) {
            self.focus = Focus::Tools;
            return;
        }
        let tools = self.tools_in_selected_category();
        if let Some(idx) = self.tool_state.selected() {
            if let Some(name) = tools.get(idx) {
                self.selected_tool = Some(name.clone());
            }
        }
    }

    /// Last tool the user activated with Enter. Cleared by callers
    /// once the detail-view (P2.04) consumes it.
    #[must_use]
    pub fn take_selected_tool(&mut self) -> Option<String> {
        self.selected_tool.take()
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q') | KeyCode::Esc, _)
            | (KeyCode::Char('c' | 'd'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Tab | KeyCode::BackTab, _) => {
                self.focus = self.focus.toggle();
            }
            (KeyCode::Char('j') | KeyCode::Down, _) => self.move_selection(1),
            (KeyCode::Char('k') | KeyCode::Up, _) => self.move_selection(-1),
            (KeyCode::Char('g') | KeyCode::Home, _) => self.move_selection(isize::MIN / 2),
            (KeyCode::Char('G') | KeyCode::End, _) => self.move_selection(isize::MAX / 2),
            (KeyCode::Char('h') | KeyCode::Left, _) => self.focus = Focus::Categories,
            (KeyCode::Char('l') | KeyCode::Right, _) => self.focus = Focus::Tools,
            (KeyCode::Enter, _) => self.confirm(),
            // Reserved for later WPs:
            // `/` → fuzzy-search overlay (P2.05)
            // `?` → in-app help overlay (P2.06)
            _ => {}
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);
        let muted = Style::default().fg(palette.muted).bg(palette.background);

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        // Header.
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
        frame.render_widget(header, vertical[0]);

        // Body — two panes side-by-side.
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(0)])
            .split(vertical[1]);

        self.render_categories(frame, body[0]);
        self.render_tools(frame, body[1]);

        // Footer.
        let footer = Paragraph::new(self.footer_line())
            .alignment(Alignment::Center)
            .style(chrome);
        frame.render_widget(footer, vertical[2]);
    }

    fn render_categories(&mut self, frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
        let palette = self.palette;
        let focused = matches!(self.focus, Focus::Categories);
        let border_style = if focused {
            Style::default().fg(palette.accent).bg(palette.background)
        } else {
            Style::default().fg(palette.muted).bg(palette.background)
        };

        let rows: Vec<ListItem<'_>> = self
            .categories
            .iter()
            .map(|name| {
                let count = self.index.category_count(name);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(palette.foreground)
                            .bg(palette.background),
                    ),
                    Span::styled(
                        format!("  ({count})"),
                        Style::default().fg(palette.muted).bg(palette.background),
                    ),
                ]))
            })
            .collect();

        let list = List::new(rows)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" categories ")
                    .style(border_style),
            )
            .highlight_style(
                Style::default()
                    .fg(palette.background)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        frame.render_stateful_widget(list, area, &mut self.cat_state);
    }

    fn render_tools(&mut self, frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
        let palette = self.palette;
        let focused = matches!(self.focus, Focus::Tools);
        let border_style = if focused {
            Style::default().fg(palette.accent).bg(palette.background)
        } else {
            Style::default().fg(palette.muted).bg(palette.background)
        };

        let tools = self.tools_in_selected_category();
        let title = self
            .selected_category()
            .map_or_else(|| " tools ".to_owned(), |c| format!(" {c} "));

        if tools.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "no tools in this category yet",
                Style::default().fg(palette.muted).bg(palette.background),
            )))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(border_style),
            );
            frame.render_widget(empty, area);
            return;
        }

        let rows: Vec<ListItem<'_>> = tools
            .iter()
            .map(|name| {
                let summary = self
                    .index
                    .get(name)
                    .map(|p| p.summary.as_str())
                    .unwrap_or_default();
                ListItem::new(Line::from(vec![
                    Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(palette.foreground)
                            .bg(palette.background)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        summary.to_owned(),
                        Style::default().fg(palette.muted).bg(palette.background),
                    ),
                ]))
            })
            .collect();

        let list = List::new(rows)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(border_style),
            )
            .highlight_style(
                Style::default()
                    .fg(palette.background)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        frame.render_stateful_widget(list, area, &mut self.tool_state);
    }

    fn footer_line(&self) -> Line<'_> {
        let palette = self.palette;
        let key = Style::default()
            .fg(palette.foreground)
            .bg(palette.background)
            .add_modifier(Modifier::BOLD);
        let muted = Style::default().fg(palette.muted).bg(palette.background);
        Line::from(vec![
            Span::styled("Tab", key),
            Span::styled(" focus  ·  ", muted),
            Span::styled("j/k ↑↓", key),
            Span::styled(" move  ·  ", muted),
            Span::styled("Enter", key),
            Span::styled(" open  ·  ", muted),
            Span::styled("q", key),
            Span::styled(" quit", muted),
        ])
    }
}

/// Initialise the terminal, run the event loop, and restore on exit.
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
    use std::fs;
    use tempfile::TempDir;

    use super::{App, Focus};
    use crate::theme::Palette;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn write_page(root: &std::path::Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    fn page(name: &str, category: &str) -> String {
        format!(
            "+++\nname = \"{name}\"\ncategory = \"{category}\"\nsummary = \"{name} summary.\"\n+++\n"
        )
    }

    fn two_category_index() -> Index {
        let dir = TempDir::new().unwrap();
        write_page(
            dir.path(),
            "file-listing/eza.md",
            &page("eza", "file-listing"),
        );
        write_page(
            dir.path(),
            "file-listing/exa.md",
            &page("exa", "file-listing"),
        );
        write_page(
            dir.path(),
            "process-management/procs.md",
            &page("procs", "process-management"),
        );
        let pages = MarkdownPagesIngestor::new(dir.path()).ingest().unwrap();
        Index::build(pages).unwrap()
    }

    fn empty_index() -> Index {
        let dir = TempDir::new().unwrap();
        let pages = MarkdownPagesIngestor::new(dir.path()).ingest().unwrap();
        Index::build(pages).unwrap()
    }

    #[test]
    fn fresh_app_focuses_categories_and_selects_first() {
        let app = App::new(two_category_index(), Palette::monochrome());
        assert_eq!(app.focus, Focus::Categories);
        assert_eq!(app.cat_state.selected(), Some(0));
        assert_eq!(app.tool_state.selected(), Some(0));
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Tools);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Categories);
    }

    #[test]
    fn j_and_down_move_category_selection() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.cat_state.selected(), Some(1));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.cat_state.selected(), Some(0));
    }

    #[test]
    fn changing_category_resets_tool_selection_to_first() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        // Move down within tools first, then change category.
        app.handle_key(key(KeyCode::Tab)); // focus → tools
        app.handle_key(key(KeyCode::Char('j'))); // tool 0 → 1
        assert_eq!(app.tool_state.selected(), Some(1));

        app.handle_key(key(KeyCode::Char('h'))); // focus → categories
        app.handle_key(key(KeyCode::Char('j'))); // next category
        // Category has only one tool (procs); tool index must clamp.
        assert_eq!(app.tool_state.selected(), Some(0));
    }

    #[test]
    fn enter_promotes_to_tools_when_focused_on_categories() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.focus, Focus::Tools);
        // Selected tool not yet recorded — Enter on categories only shifts focus.
        assert!(app.selected_tool.is_none());
    }

    #[test]
    fn enter_in_tools_records_selection() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Enter));
        // First tool in the first category is "exa" or "eza" (sort order).
        let picked = app.take_selected_tool();
        assert!(matches!(picked.as_deref(), Some("eza" | "exa")));
    }

    #[test]
    fn empty_index_has_no_selection() {
        let app = App::new(empty_index(), Palette::monochrome());
        assert_eq!(app.cat_state.selected(), None);
        assert_eq!(app.tool_state.selected(), None);
    }

    #[test]
    fn q_and_ctrl_c_still_quit() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
        let mut app2 = App::new(two_category_index(), Palette::monochrome());
        app2.handle_key(ctrl(KeyCode::Char('c')));
        assert!(app2.should_quit);
    }

    #[test]
    fn g_jumps_to_first_and_shift_g_to_last() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('j'))); // 0 → 1
        app.handle_key(key(KeyCode::Char('g'))); // → 0
        assert_eq!(app.cat_state.selected(), Some(0));
        app.handle_key(key(KeyCode::Char('G'))); // → last
        assert_eq!(app.cat_state.selected(), Some(app.categories.len() - 1));
    }
}
