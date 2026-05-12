// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Minimal Markdown → `Vec<Line<'static>>` renderer for the TUI body
//! pane.
//!
//! Walks `pulldown-cmark` events and produces styled lines. The
//! styling tier is intentionally small — bold for headings, accent
//! for code spans / code blocks, muted for block quotes, plain
//! foreground everywhere else. The TUI is for browsing, not
//! formatting fidelity; the curated body's prose has to read well
//! regardless of styling.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Palette;

/// Render `markdown` into a sequence of styled lines coloured against
/// `palette`. Every returned line owns its strings so the caller can
/// stick them in a `Paragraph` without further lifetime juggling.
#[must_use]
pub(crate) fn render(markdown: &str, palette: Palette) -> Vec<Line<'static>> {
    let mut renderer = Renderer::new(palette);
    let parser = Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH);
    for event in parser {
        renderer.handle(event);
    }
    renderer.finish()
}

struct Renderer {
    palette: Palette,
    lines: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    list_depth: usize,
    in_heading: bool,
    in_code_block: bool,
}

impl Renderer {
    fn new(palette: Palette) -> Self {
        Self {
            palette,
            lines: Vec::new(),
            current: Vec::new(),
            style_stack: Vec::new(),
            list_depth: 0,
            in_heading: false,
            in_code_block: false,
        }
    }

    fn base_style(&self) -> Style {
        Style::default()
            .fg(self.palette.foreground)
            .bg(self.palette.background)
    }

    fn current_style(&self) -> Style {
        self.style_stack
            .last()
            .copied()
            .unwrap_or_else(|| self.base_style())
    }

    fn push_text(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        let style = self.current_style();
        self.current.push(Span::styled(text, style));
    }

    fn flush_line(&mut self) {
        let spans = std::mem::take(&mut self.current);
        self.lines.push(Line::from(spans));
    }

    fn blank_line(&mut self) {
        if !self.current.is_empty() {
            self.flush_line();
        }
        // Only emit a blank line if the last one wasn't blank.
        let last_blank = self
            .lines
            .last()
            .is_none_or(|l| l.spans.iter().all(|s| s.content.is_empty()));
        if !last_blank {
            self.lines.push(Line::from(""));
        }
    }

    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.push_text(text.into_string()),
            Event::Code(text) => {
                let style = self
                    .base_style()
                    .fg(self.palette.accent)
                    .add_modifier(Modifier::BOLD);
                self.current.push(Span::styled(format!("`{text}`"), style));
            }
            Event::SoftBreak | Event::HardBreak => self.flush_line(),
            Event::Rule => {
                self.blank_line();
                let style = self.base_style().fg(self.palette.muted);
                self.current.push(Span::styled("──────", style));
                self.flush_line();
                self.blank_line();
            }
            Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::DisplayMath(_)
            | Event::InlineMath(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                if !self.current.is_empty() {
                    self.flush_line();
                }
            }
            Tag::Heading { level, .. } => {
                self.blank_line();
                let style = self
                    .base_style()
                    .fg(self.palette.accent)
                    .add_modifier(Modifier::BOLD);
                self.style_stack.push(style);
                let marker = "#".repeat(heading_depth(level));
                self.push_text(format!("{marker} "));
                self.in_heading = true;
            }
            Tag::BlockQuote(_) => {
                self.blank_line();
                let style = self.base_style().fg(self.palette.muted);
                self.style_stack.push(style);
                self.push_text("│ ".to_owned());
            }
            Tag::CodeBlock(kind) => {
                self.blank_line();
                let label = match kind {
                    CodeBlockKind::Fenced(info) if !info.is_empty() => {
                        format!("‹ {info} ›")
                    }
                    _ => "‹ code ›".to_owned(),
                };
                let header_style = self.base_style().fg(self.palette.muted);
                self.current.push(Span::styled(label, header_style));
                self.flush_line();
                let style = self
                    .base_style()
                    .fg(self.palette.accent)
                    .add_modifier(Modifier::BOLD);
                self.style_stack.push(style);
                self.in_code_block = true;
            }
            Tag::List(_) => {
                self.list_depth += 1;
                self.blank_line();
            }
            Tag::Item => {
                let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                self.push_text(format!("{indent}• "));
            }
            Tag::Emphasis => {
                let style = self.current_style().add_modifier(Modifier::ITALIC);
                self.style_stack.push(style);
            }
            Tag::Strong => {
                let style = self.current_style().add_modifier(Modifier::BOLD);
                self.style_stack.push(style);
            }
            Tag::Strikethrough => {
                let style = self.current_style().add_modifier(Modifier::CROSSED_OUT);
                self.style_stack.push(style);
            }
            Tag::Link { dest_url, .. } => {
                let style = self.current_style().add_modifier(Modifier::UNDERLINED);
                self.style_stack.push(style);
                // Record the URL inline so the user can still see it.
                self.push_text(format!("[link → {dest_url}] "));
            }
            Tag::Image { dest_url, .. } => {
                let style = self.current_style().fg(self.palette.muted);
                self.style_stack.push(style);
                self.push_text(format!("[image: {dest_url}] "));
            }
            // Unsupported / structural tags: we still match exhaustively
            // so future pulldown-cmark variants surface as compile
            // errors rather than silent drops.
            Tag::HtmlBlock
            | Tag::FootnoteDefinition(_)
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::MetadataBlock(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Superscript
            | Tag::Subscript => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_line();
                self.blank_line();
            }
            TagEnd::Heading(_) => {
                if self.in_heading {
                    self.style_stack.pop();
                    self.in_heading = false;
                }
                self.flush_line();
                self.blank_line();
            }
            TagEnd::BlockQuote(_) => {
                self.style_stack.pop();
                self.flush_line();
                self.blank_line();
            }
            TagEnd::CodeBlock => {
                if self.in_code_block {
                    self.style_stack.pop();
                    self.in_code_block = false;
                }
                if !self.current.is_empty() {
                    self.flush_line();
                }
                self.blank_line();
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                if !self.current.is_empty() {
                    self.flush_line();
                }
                self.blank_line();
            }
            TagEnd::Item => {
                self.flush_line();
            }
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Link
            | TagEnd::Image => {
                self.style_stack.pop();
            }
            _ => {}
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        if !self.current.is_empty() {
            self.flush_line();
        }
        self.lines
    }
}

fn heading_depth(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::theme::Palette;

    #[test]
    fn empty_input_renders_no_lines() {
        let out = render("", Palette::monochrome());
        assert!(out.is_empty());
    }

    #[test]
    fn paragraph_round_trips_visible_text() {
        let out = render("hello world", Palette::monochrome());
        let rendered: String = out
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains("hello world"));
    }

    #[test]
    fn heading_gets_marker_prefix() {
        let out = render("## Section\n", Palette::monochrome());
        let rendered: String = out
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains("## Section"));
    }

    #[test]
    fn fenced_code_block_emits_label() {
        let out = render("```sh\nls -la\n```\n", Palette::monochrome());
        let rendered: String = out
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains("‹ sh ›"));
        assert!(rendered.contains("ls -la"));
    }

    #[test]
    fn bullet_list_uses_bullet_glyph() {
        let out = render("- one\n- two\n", Palette::monochrome());
        let rendered: String = out
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains("• one"));
        assert!(rendered.contains("• two"));
    }

    #[test]
    fn link_inlines_destination() {
        let out = render("see [docs](https://example.com)\n", Palette::monochrome());
        let rendered: String = out
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(rendered.contains("https://example.com"));
        assert!(rendered.contains("docs"));
    }
}
