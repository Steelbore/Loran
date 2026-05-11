// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Plain-text Markdown renderer driving the Phase 1 CLI output paths.

use std::io::{self, Write};

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// Render `body_md` as plain text into `writer`.
///
/// The output contains no ANSI escape codes and is safe to pipe through
/// POSIX text utilities. See the crate-level docs for the exact mapping
/// from `CommonMark` constructs to terminal output.
///
/// # Errors
///
/// Returns any [`io::Error`] surfaced by `writer`. Markdown parse
/// problems do not produce an error — `pulldown-cmark` is permissive and
/// emits the offending text as a `Text` event rather than failing.
pub fn render_text<W: Write>(body_md: &str, writer: &mut W) -> io::Result<()> {
    let parser = Parser::new(body_md);
    let mut state = RenderState::new();

    for event in parser {
        state.handle(event, writer)?;
    }
    state.flush_trailing(writer)
}

/// Mutable state threaded through the event loop.
struct RenderState {
    list_depth: usize,
    in_code_block: bool,
    /// Where the current line of output is being assembled — appended to
    /// by `Text` / `Code` / link-flush; flushed on paragraph / heading /
    /// item ends. Empty between blocks.
    line_buffer: String,
    /// Captures the URL of a link whose anchor text we are currently
    /// emitting. Appended after the closing `End(Link)`.
    pending_link_url: Option<String>,
    /// `true` immediately after a block-level emit that already wrote
    /// its own trailing blank line. Used to coalesce duplicate blanks.
    just_wrote_blank: bool,
    /// Number of leading blank lines pending at the front of the
    /// document — suppressed in the final flush.
    document_started: bool,
}

impl RenderState {
    fn new() -> Self {
        Self {
            list_depth: 0,
            in_code_block: false,
            line_buffer: String::new(),
            pending_link_url: None,
            just_wrote_blank: true,
            document_started: false,
        }
    }

    fn handle<W: Write>(&mut self, event: Event<'_>, writer: &mut W) -> io::Result<()> {
        match event {
            Event::Start(tag) => self.start(tag, writer),
            Event::End(tag) => self.end(tag, writer),
            Event::Text(text) => {
                if self.in_code_block {
                    self.emit_code_block_text(&text, writer)
                } else {
                    self.line_buffer.push_str(&text);
                    Ok(())
                }
            }
            Event::Code(text) => {
                self.line_buffer.push('`');
                self.line_buffer.push_str(&text);
                self.line_buffer.push('`');
                Ok(())
            }
            Event::Html(text) | Event::InlineHtml(text) => {
                self.line_buffer.push_str(&text);
                Ok(())
            }
            Event::SoftBreak => {
                self.line_buffer.push(' ');
                Ok(())
            }
            Event::HardBreak => {
                self.flush_line(writer)?;
                Ok(())
            }
            Event::Rule => {
                self.flush_line(writer)?;
                self.write_blank_line(writer)?;
                writer.write_all(b"---\n")?;
                self.write_blank_line(writer)?;
                Ok(())
            }
            Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => Ok(()),
        }
    }

    fn start<W: Write>(&mut self, tag: Tag<'_>, writer: &mut W) -> io::Result<()> {
        match tag {
            Tag::Heading { .. } => {
                self.flush_line(writer)?;
                self.ensure_block_separation(writer)
            }
            Tag::Paragraph => {
                self.flush_line(writer)?;
                self.ensure_block_separation(writer)
            }
            Tag::List(_) => {
                self.flush_line(writer)?;
                if self.list_depth == 0 {
                    self.ensure_block_separation(writer)?;
                }
                self.list_depth += 1;
                Ok(())
            }
            Tag::Item => {
                let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                self.line_buffer.push_str(&indent);
                self.line_buffer.push_str("- ");
                Ok(())
            }
            Tag::CodeBlock(_) => {
                self.flush_line(writer)?;
                self.ensure_block_separation(writer)?;
                self.in_code_block = true;
                Ok(())
            }
            Tag::Link { dest_url, .. } => {
                self.pending_link_url = Some(dest_url.into_string());
                Ok(())
            }
            // Unimplemented tags: emit nothing for the opener and let
            // the matching End reset state. Tables / footnotes / images
            // / strikethrough are not used in the v1 catalog content.
            _ => Ok(()),
        }
    }

    fn end<W: Write>(&mut self, tag: TagEnd, writer: &mut W) -> io::Result<()> {
        match tag {
            TagEnd::Heading(level) => {
                let upper = heading_text(&self.line_buffer, level);
                self.line_buffer.clear();
                self.line_buffer.push_str(&upper);
                self.flush_line(writer)?;
                self.write_blank_line(writer)
            }
            TagEnd::Paragraph => {
                self.flush_line(writer)?;
                self.write_blank_line(writer)
            }
            TagEnd::List(_) => {
                self.flush_line(writer)?;
                self.list_depth = self.list_depth.saturating_sub(1);
                if self.list_depth == 0 {
                    self.write_blank_line(writer)?;
                }
                Ok(())
            }
            TagEnd::Item => self.flush_line(writer),
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.write_blank_line(writer)
            }
            TagEnd::Link => {
                if let Some(url) = self.pending_link_url.take() {
                    self.line_buffer.push_str(" (");
                    self.line_buffer.push_str(&url);
                    self.line_buffer.push(')');
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Indent every line of a code-block text chunk by four spaces and
    /// emit directly to the writer (bypassing `line_buffer`, because
    /// code blocks may contain multiple internal newlines).
    fn emit_code_block_text<W: Write>(&mut self, text: &str, writer: &mut W) -> io::Result<()> {
        for line in text.split_inclusive('\n') {
            writer.write_all(b"    ")?;
            writer.write_all(line.as_bytes())?;
        }
        self.just_wrote_blank = false;
        self.document_started = true;
        Ok(())
    }

    fn flush_line<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if self.line_buffer.is_empty() {
            return Ok(());
        }
        writer.write_all(self.line_buffer.as_bytes())?;
        writer.write_all(b"\n")?;
        self.line_buffer.clear();
        self.just_wrote_blank = false;
        self.document_started = true;
        Ok(())
    }

    fn write_blank_line<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if !self.document_started || self.just_wrote_blank {
            return Ok(());
        }
        writer.write_all(b"\n")?;
        self.just_wrote_blank = true;
        Ok(())
    }

    fn ensure_block_separation<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if self.document_started && !self.just_wrote_blank {
            self.write_blank_line(writer)?;
        }
        Ok(())
    }

    fn flush_trailing<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.flush_line(writer)?;
        Ok(())
    }
}

/// Map a heading level to its rendered shape. v1 emits every heading
/// in UPPER CASE; the level is preserved as the count of leading "= "
/// markers prepended for visual hierarchy on monochrome terminals.
fn heading_text(raw: &str, level: HeadingLevel) -> String {
    let depth = match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    };
    let prefix = "=".repeat(depth);
    format!("{prefix} {}", raw.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::render_text;

    fn render(md: &str) -> String {
        let mut out = Vec::new();
        render_text(md, &mut out).expect("rendering must succeed");
        String::from_utf8(out).expect("rendered output is valid UTF-8")
    }

    fn contains_ansi(s: &str) -> bool {
        s.bytes().any(|b| b == 0x1b)
    }

    #[test]
    fn paragraph_only() {
        let out = render("Hello, world.\n\nSecond paragraph here.");
        assert_eq!(out, "Hello, world.\n\nSecond paragraph here.\n\n");
        assert!(!contains_ansi(&out));
    }

    #[test]
    fn headings_become_uppercase_with_marker_prefix() {
        let out = render("# Title\n\n## Subtitle\n\nBody text.");
        assert!(out.contains("= TITLE\n"));
        assert!(out.contains("== SUBTITLE\n"));
        assert!(out.contains("Body text."));
        assert!(!contains_ansi(&out));
    }

    #[test]
    fn fenced_code_block_indented_four_spaces() {
        let md = "Try this:\n\n```\ncargo build\ncargo test\n```\n";
        let out = render(md);
        assert!(out.contains("    cargo build\n"));
        assert!(out.contains("    cargo test\n"));
        assert!(!contains_ansi(&out));
    }

    #[test]
    fn lists_emit_bullets_with_indent_per_depth() {
        let md = "- top one\n- top two\n  - nested\n  - also nested\n";
        let out = render(md);
        assert!(out.contains("- top one\n"));
        assert!(out.contains("- top two\n"));
        assert!(out.contains("  - nested\n"));
        assert!(out.contains("  - also nested\n"));
        assert!(!contains_ansi(&out));
    }

    #[test]
    fn links_become_text_and_url_in_parens() {
        let out = render("See [the eza homepage](https://eza.rocks) for docs.");
        assert!(out.contains("the eza homepage (https://eza.rocks)"));
        assert!(!contains_ansi(&out));
    }

    #[test]
    fn inline_code_keeps_backticks() {
        let out = render("Use `cargo test` to run tests.");
        assert!(out.contains("`cargo test`"));
        assert!(!contains_ansi(&out));
    }

    #[test]
    fn mixed_document_renders_without_ansi() {
        let md = "\
# eza

eza is a modern `ls` replacement.

## Quickstart

```
alias ls = eza --git
```

- pairs with `bat`
- pairs with [fd](https://github.com/sharkdp/fd)
";
        let out = render(md);
        assert!(out.contains("= EZA"));
        assert!(out.contains("== QUICKSTART"));
        assert!(out.contains("    alias ls = eza --git"));
        assert!(out.contains("- pairs with `bat`"));
        assert!(out.contains("- pairs with fd (https://github.com/sharkdp/fd)"));
        assert!(!contains_ansi(&out), "rendered output must be ANSI-free");
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let out = render("");
        assert!(out.is_empty());
    }
}
