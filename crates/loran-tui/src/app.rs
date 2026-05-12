// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! TUI application loop.
//!
//! State machine:
//!
//! - `View::Browse` (WP-P2.03): dual-pane categories + tools.
//! - `View::Detail` (WP-P2.04): name + intro + body with a right
//!   sidebar of `pairs_with` / `safe_alias_for` / `written_in`
//!   badges. Tab cycles `Rendered → Raw → Frontmatter` sub-views.
//!
//! Esc returns one step (Detail → Browse, Browse → quit); `q` and
//! `Ctrl-C` always quit. `/` and `?` are reserved for the search
//! overlay (P2.05) and in-app help (P2.06).

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use loran_core::resolve_search;
use loran_index::Index;
use loran_pages::Page;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::TerminalGuard;
use crate::markdown;
use crate::terminal::TuiError;
use crate::theme::Palette;

/// Tick frequency for the event loop. 60 ms is enough for "feels
/// instant" key responsiveness without burning CPU on an idle screen.
const TICK_MS: u64 = 60;

/// Which pane currently owns keyboard focus in the browse view.
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

/// Top-level navigation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum View {
    Browse,
    Detail {
        tool: String,
        sub_view: DetailSubView,
    },
}

/// Modal overlay state. Layered on top of [`View::Browse`]; when
/// closed, the prior browse selection is preserved.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct SearchOverlay {
    pub query: String,
    pub matches: Vec<String>,
    pub selected: usize,
}

/// Detail-view sub-modes. Tab cycles `Rendered → Raw → Frontmatter`,
/// matching Spec §10's agent-inspection contract.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DetailSubView {
    Rendered,
    Raw,
    Frontmatter,
}

impl DetailSubView {
    fn cycle(self) -> Self {
        match self {
            Self::Rendered => Self::Raw,
            Self::Raw => Self::Frontmatter,
            Self::Frontmatter => Self::Rendered,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Rendered => "rendered",
            Self::Raw => "raw markdown",
            Self::Frontmatter => "frontmatter",
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
    view: View,
    /// Optional modal overlay (search). When `Some`, it captures
    /// keystrokes; when `None`, the active `view` handles them.
    search: Option<SearchOverlay>,
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
            view: View::Browse,
            search: None,
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
                self.open_detail(name.clone());
            }
        }
    }

    fn open_detail(&mut self, tool: String) {
        self.view = View::Detail {
            tool,
            sub_view: DetailSubView::Rendered,
        };
    }

    fn close_detail(&mut self) {
        self.view = View::Browse;
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Ctrl-C / Ctrl-D always quit; `q` only when not typing in a
        // search overlay (so a query containing `q` stays intact).
        if matches!(key.modifiers, KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'd'))
        {
            self.should_quit = true;
            return;
        }
        if self.search.is_none()
            && matches!(key.code, KeyCode::Char('q'))
            && !matches!(key.modifiers, KeyModifiers::SHIFT)
        {
            self.should_quit = true;
            return;
        }

        if self.search.is_some() {
            self.handle_search_key(key);
            return;
        }

        match self.view.clone() {
            View::Browse => self.handle_browse_key(key),
            View::Detail { tool, sub_view } => self.handle_detail_key(key, tool, sub_view),
        }
    }

    fn open_search(&mut self) {
        let mut overlay = SearchOverlay::default();
        self.refresh_search_matches(&mut overlay);
        self.search = Some(overlay);
    }

    fn refresh_search_matches(&self, overlay: &mut SearchOverlay) {
        let trimmed = overlay.query.trim();
        if trimmed.is_empty() {
            // Empty query: show every page sorted by name so the user
            // sees the catalog while typing the first character.
            overlay.matches = {
                let mut names: Vec<String> = self.index.all().map(|p| p.name.clone()).collect();
                names.sort();
                names
            };
        } else {
            let result = resolve_search(&self.index, trimmed);
            overlay.matches = result.matches.into_iter().map(|m| m.page.name).collect();
        }
        if overlay.matches.is_empty() {
            overlay.selected = 0;
        } else {
            overlay.selected = overlay.selected.min(overlay.matches.len() - 1);
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        // Take the overlay out so we can mutate it without aliasing
        // `self`, then put it back unless the user dismissed it.
        let Some(mut overlay) = self.search.take() else {
            return;
        };
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                // Drop overlay; return to whatever view was beneath.
                return;
            }
            (KeyCode::Enter, _) => {
                if let Some(name) = overlay.matches.get(overlay.selected).cloned() {
                    self.open_detail(name);
                    return; // overlay closed
                }
                self.search = Some(overlay);
                return;
            }
            (KeyCode::Backspace, _) => {
                overlay.query.pop();
                self.refresh_search_matches(&mut overlay);
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                if !overlay.matches.is_empty() {
                    overlay.selected = (overlay.selected + 1).min(overlay.matches.len() - 1);
                }
            }
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                overlay.selected = overlay.selected.saturating_sub(1);
            }
            (KeyCode::Char(c), modifiers)
                if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
            {
                overlay.query.push(c);
                self.refresh_search_matches(&mut overlay);
            }
            _ => {}
        }
        self.search = Some(overlay);
    }

    fn handle_browse_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => self.should_quit = true,
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
            (KeyCode::Char('/'), _) => self.open_search(),
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent, tool: String, sub_view: DetailSubView) {
        match (key.code, key.modifiers) {
            (KeyCode::Esc | KeyCode::Backspace, _) => self.close_detail(),
            (KeyCode::Tab, _) => {
                self.view = View::Detail {
                    tool,
                    sub_view: sub_view.cycle(),
                };
            }
            _ => {}
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
        match self.view.clone() {
            View::Browse => self.render_browse(frame),
            View::Detail { tool, sub_view } => self.render_detail(frame, &tool, sub_view),
        }
        if let Some(overlay) = self.search.clone() {
            self.render_search_overlay(frame, &overlay);
        }
    }

    fn render_search_overlay(&self, frame: &mut Frame<'_>, overlay: &SearchOverlay) {
        let area = centered_rect(frame.area(), 70, 60);
        frame.render_widget(Clear, area);

        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);
        let muted = Style::default().fg(palette.muted).bg(palette.background);
        let accent = Style::default().fg(palette.accent).bg(palette.background);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" search ")
            .style(chrome);
        frame.render_widget(block, area);

        let inner = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);

        // Query line.
        let query = Paragraph::new(Line::from(vec![
            Span::styled("› ".to_owned(), accent.add_modifier(Modifier::BOLD)),
            Span::styled(overlay.query.clone(), chrome),
            Span::styled("█".to_owned(), accent),
        ]))
        .style(chrome)
        .block(Block::default().borders(Borders::BOTTOM).style(muted));
        frame.render_widget(query, chunks[0]);

        // Match list.
        if overlay.matches.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled("no matches".to_owned(), muted)))
                .alignment(Alignment::Center)
                .style(chrome);
            frame.render_widget(empty, chunks[1]);
        } else {
            let rows: Vec<ListItem<'_>> = overlay
                .matches
                .iter()
                .enumerate()
                .map(|(idx, name)| {
                    let summary = self
                        .index
                        .get(name)
                        .map(|p| p.summary.as_str())
                        .unwrap_or_default();
                    let marker = if idx == overlay.selected {
                        "▸ "
                    } else {
                        "  "
                    };
                    let primary = if idx == overlay.selected {
                        accent.add_modifier(Modifier::BOLD)
                    } else {
                        chrome
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(marker.to_owned(), primary),
                        Span::styled(name.clone(), primary),
                        Span::raw("  "),
                        Span::styled(summary.to_owned(), muted),
                    ]))
                })
                .collect();
            let list = List::new(rows).style(chrome);
            frame.render_widget(list, chunks[1]);
        }

        // Footer hint.
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("type", accent),
                Span::styled(" filter  ·  ", muted),
                Span::styled("↑/↓", accent),
                Span::styled(" move  ·  ", muted),
                Span::styled("Enter", accent),
                Span::styled(" open  ·  ", muted),
                Span::styled("Esc", accent),
                Span::styled(" cancel", muted),
            ]))
            .style(chrome)
            .alignment(Alignment::Center),
            chunks[2],
        );
    }

    fn render_browse(&mut self, frame: &mut Frame<'_>) {
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

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(0)])
            .split(vertical[1]);

        self.render_categories(frame, body[0]);
        self.render_tools(frame, body[1]);

        frame.render_widget(
            Paragraph::new(self.footer_browse())
                .alignment(Alignment::Center)
                .style(chrome),
            vertical[2],
        );
    }

    fn render_categories(&mut self, frame: &mut Frame<'_>, area: Rect) {
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

    fn render_tools(&mut self, frame: &mut Frame<'_>, area: Rect) {
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

    fn render_detail(&mut self, frame: &mut Frame<'_>, tool: &str, sub_view: DetailSubView) {
        let area = frame.area();
        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        // Header: tool name + sub-view label.
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                tool.to_uppercase(),
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                format!("[{}]", sub_view.label()),
                Style::default().fg(palette.muted).bg(palette.background),
            ),
        ]))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::BOTTOM).style(chrome));
        frame.render_widget(header, vertical[0]);

        // Body + sidebar.
        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(28)])
            .split(vertical[1]);

        let page = self.index.get(tool);
        self.render_detail_body(frame, mid[0], tool, page, sub_view);
        self.render_detail_sidebar(frame, mid[1], page);

        // Footer.
        frame.render_widget(
            Paragraph::new(self.footer_detail())
                .alignment(Alignment::Center)
                .style(chrome),
            vertical[2],
        );
    }

    fn render_detail_body(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        tool: &str,
        page: Option<&Page>,
        sub_view: DetailSubView,
    ) {
        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);
        let muted = Style::default().fg(palette.muted).bg(palette.background);

        let Some(page) = page else {
            let missing = Paragraph::new(Line::from(Span::styled(
                format!("page `{tool}` not in the merged index"),
                muted,
            )))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).style(chrome));
            frame.render_widget(missing, area);
            return;
        };

        let title = match sub_view {
            DetailSubView::Rendered => " rendered ",
            DetailSubView::Raw => " raw ",
            DetailSubView::Frontmatter => " frontmatter ",
        };

        let mut lines: Vec<Line<'static>> = vec![
            Line::from(Span::styled(page.summary.clone(), chrome)),
            Line::from(Span::styled(
                "Steelbore curated entry · Tab to cycle views".to_owned(),
                muted,
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "──────".to_owned(),
                Style::default().fg(palette.muted).bg(palette.background),
            )),
            Line::raw(""),
        ];

        match sub_view {
            DetailSubView::Rendered => {
                lines.extend(markdown::render(&page.body, palette));
            }
            DetailSubView::Raw => {
                for line in page.body.lines() {
                    lines.push(Line::from(Span::styled(line.to_owned(), chrome)));
                }
            }
            DetailSubView::Frontmatter => {
                lines.extend(frontmatter_lines(page, palette));
            }
        }

        let body = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(chrome),
        );
        frame.render_widget(body, area);
    }

    fn render_detail_sidebar(&self, frame: &mut Frame<'_>, area: Rect, page: Option<&Page>) {
        let palette = self.palette;
        let chrome = Style::default()
            .fg(palette.foreground)
            .bg(palette.background);
        let muted = Style::default().fg(palette.muted).bg(palette.background);
        let label = Style::default().fg(palette.muted).bg(palette.background);
        let accent = Style::default()
            .fg(palette.accent)
            .bg(palette.background)
            .add_modifier(Modifier::BOLD);

        let mut lines: Vec<Line<'static>> = Vec::new();

        if let Some(page) = page {
            // Category.
            lines.push(Line::from(vec![
                Span::styled("category ".to_owned(), label),
                Span::styled(page.category.clone(), chrome),
            ]));

            // written_in with rust badge.
            if let Some(lang) = &page.written_in {
                let badge = if lang.eq_ignore_ascii_case("rust") {
                    "🦀 rust".to_owned()
                } else {
                    lang.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled("written_in ".to_owned(), label),
                    Span::styled(badge, accent),
                ]));
            }

            if !page.replaces.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::from(Span::styled("replaces".to_owned(), label)));
                for r in &page.replaces {
                    lines.push(Line::from(Span::styled(format!("· {r}"), chrome)));
                }
            }

            if !page.safe_alias_for.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::from(Span::styled("safe_alias_for".to_owned(), label)));
                for a in &page.safe_alias_for {
                    lines.push(Line::from(Span::styled(format!("✓ {a}"), accent)));
                }
            }

            if !page.pairs_with.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::from(Span::styled("pairs_with".to_owned(), label)));
                for p in &page.pairs_with {
                    lines.push(Line::from(Span::styled(format!("+ {p}"), chrome)));
                }
            }

            if !page.tags.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::from(Span::styled("tags".to_owned(), label)));
                lines.push(Line::from(Span::styled(page.tags.join(", "), muted)));
            }

            if let Some(official) = &page.official {
                lines.push(Line::raw(""));
                lines.push(Line::from(Span::styled("official".to_owned(), label)));
                lines.push(Line::from(Span::styled(official.clone(), muted)));
            }
        }

        let sidebar = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" badges ")
                .style(chrome),
        );
        frame.render_widget(sidebar, area);
    }

    fn footer_browse(&self) -> Line<'_> {
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
            Span::styled("/", key),
            Span::styled(" search  ·  ", muted),
            Span::styled("q", key),
            Span::styled(" quit", muted),
        ])
    }

    fn footer_detail(&self) -> Line<'_> {
        let palette = self.palette;
        let key = Style::default()
            .fg(palette.foreground)
            .bg(palette.background)
            .add_modifier(Modifier::BOLD);
        let muted = Style::default().fg(palette.muted).bg(palette.background);
        Line::from(vec![
            Span::styled("Tab", key),
            Span::styled(" cycle views  ·  ", muted),
            Span::styled("Esc", key),
            Span::styled(" back  ·  ", muted),
            Span::styled("q", key),
            Span::styled(" quit", muted),
        ])
    }
}

fn frontmatter_lines(page: &Page, palette: Palette) -> Vec<Line<'static>> {
    let chrome = Style::default()
        .fg(palette.foreground)
        .bg(palette.background);
    let label = Style::default().fg(palette.muted).bg(palette.background);
    let mut lines = Vec::new();

    let push = |lines: &mut Vec<Line<'static>>, key: &str, value: String| {
        lines.push(Line::from(vec![
            Span::styled(format!("{key:<14} = "), label),
            Span::styled(value, chrome),
        ]));
    };

    push(&mut lines, "name", quote(&page.name));
    push(&mut lines, "category", quote(&page.category));
    push(&mut lines, "summary", quote(&page.summary));
    push(&mut lines, "replaces", toml_array(&page.replaces));
    push(
        &mut lines,
        "safe_alias_for",
        toml_array(&page.safe_alias_for),
    );
    push(&mut lines, "pairs_with", toml_array(&page.pairs_with));
    push(&mut lines, "tags", toml_array(&page.tags));
    push(&mut lines, "aliases", toml_array(&page.aliases));
    if let Some(v) = &page.official {
        push(&mut lines, "official", quote(v));
    }
    if let Some(v) = &page.tldr_page {
        push(&mut lines, "tldr_page", quote(v));
    }
    if let Some(v) = &page.written_in {
        push(&mut lines, "written_in", quote(v));
    }
    if let Some(v) = &page.since {
        push(&mut lines, "since", quote(v));
    }
    lines
}

fn quote(value: &str) -> String {
    format!("\"{value}\"")
}

/// Centre a rectangle of `pct_x` % width × `pct_y` % height inside
/// `outer`. Used to position modal overlays.
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

fn toml_array(items: &[String]) -> String {
    let joined = items
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{joined}]")
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
    use std::fs;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use loran_index::{Index, Ingestor, MarkdownPagesIngestor};
    use tempfile::TempDir;

    use super::{App, DetailSubView, Focus, View};
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

    fn rich_page(name: &str) -> String {
        format!(
            "+++\n\
             name = \"{name}\"\n\
             category = \"file-listing\"\n\
             summary = \"{name} summary.\"\n\
             replaces = [\"ls\"]\n\
             safe_alias_for = [\"ls\"]\n\
             pairs_with = [\"bat\"]\n\
             tags = [\"x\"]\n\
             written_in = \"rust\"\n\
             +++\n\
             \n\
             ## {name}\n\n\
             Steelbore notes for {name}.\n"
        )
    }

    fn two_category_index() -> Index {
        let dir = TempDir::new().unwrap();
        write_page(dir.path(), "file-listing/eza.md", &rich_page("eza"));
        write_page(dir.path(), "file-listing/exa.md", &rich_page("exa"));
        write_page(
            dir.path(),
            "process-management/procs.md",
            "+++\nname = \"procs\"\ncategory = \"process-management\"\n\
             summary = \"procs.\"\n+++\n",
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
        assert_eq!(app.view, View::Browse);
    }

    #[test]
    fn tab_toggles_focus_in_browse() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Tools);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Categories);
    }

    #[test]
    fn enter_in_tools_opens_detail_view() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Enter));
        match &app.view {
            View::Detail { tool, sub_view } => {
                assert!(matches!(tool.as_str(), "eza" | "exa"));
                assert_eq!(*sub_view, DetailSubView::Rendered);
            }
            View::Browse => panic!("expected Detail view, got Browse"),
        }
    }

    #[test]
    fn tab_in_detail_cycles_sub_views() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Tab));
        let View::Detail { sub_view, .. } = &app.view else {
            panic!("expected Detail view");
        };
        assert_eq!(*sub_view, DetailSubView::Raw);

        app.handle_key(key(KeyCode::Tab));
        let View::Detail { sub_view, .. } = &app.view else {
            panic!("expected Detail view");
        };
        assert_eq!(*sub_view, DetailSubView::Frontmatter);

        app.handle_key(key(KeyCode::Tab));
        let View::Detail { sub_view, .. } = &app.view else {
            panic!("expected Detail view");
        };
        assert_eq!(*sub_view, DetailSubView::Rendered);
    }

    #[test]
    fn esc_in_detail_returns_to_browse() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(app.view, View::Detail { .. }));

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.view, View::Browse);
    }

    #[test]
    fn q_quits_from_any_view() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_quits_from_detail() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(ctrl(KeyCode::Char('c')));
        assert!(app.should_quit);
    }

    #[test]
    fn esc_quits_from_browse() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Esc));
        assert!(app.should_quit);
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
    fn empty_index_has_no_selection() {
        let app = App::new(empty_index(), Palette::monochrome());
        assert_eq!(app.cat_state.selected(), None);
        assert_eq!(app.tool_state.selected(), None);
    }

    #[test]
    fn slash_opens_search_overlay() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        assert!(app.search.is_some());
        // Empty query pre-populates with all tools sorted.
        let overlay = app.search.as_ref().unwrap();
        assert!(!overlay.matches.is_empty());
        assert_eq!(overlay.query, "");
    }

    #[test]
    fn search_typing_filters_results() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Char('z')));
        app.handle_key(key(KeyCode::Char('a')));
        let overlay = app.search.as_ref().unwrap();
        assert_eq!(overlay.query, "eza");
        assert!(overlay.matches.iter().any(|n| n == "eza"));
    }

    #[test]
    fn search_q_in_query_does_not_quit() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('q')));
        assert!(!app.should_quit, "`q` inside the query must stay literal");
        assert_eq!(app.search.as_ref().unwrap().query, "q");
    }

    #[test]
    fn search_esc_closes_overlay_without_quitting() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Esc));
        assert!(app.search.is_none());
        assert!(!app.should_quit);
        assert_eq!(app.view, View::Browse);
    }

    #[test]
    fn search_enter_opens_detail_for_selected_match() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Char('z')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.search.is_none());
        match &app.view {
            View::Detail { tool, .. } => assert_eq!(tool, "eza"),
            View::Browse => panic!("expected Detail view, got Browse"),
        }
    }

    #[test]
    fn search_arrow_keys_move_selection() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        let initial = app.search.as_ref().unwrap().selected;
        app.handle_key(key(KeyCode::Down));
        let after = app.search.as_ref().unwrap().selected;
        assert_eq!(after, initial + 1);
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.search.as_ref().unwrap().selected, initial);
    }

    #[test]
    fn search_backspace_pops_query_chars() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Char('z')));
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.search.as_ref().unwrap().query, "e");
    }

    #[test]
    fn ctrl_c_in_search_still_quits() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(ctrl(KeyCode::Char('c')));
        assert!(app.should_quit);
    }

    #[test]
    fn g_jumps_to_first_and_shift_g_to_last() {
        let mut app = App::new(two_category_index(), Palette::monochrome());
        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('g')));
        assert_eq!(app.cat_state.selected(), Some(0));
        app.handle_key(key(KeyCode::Char('G')));
        assert_eq!(app.cat_state.selected(), Some(app.categories.len() - 1));
    }
}
