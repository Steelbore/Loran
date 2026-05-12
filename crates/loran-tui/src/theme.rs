// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Steelbore palette tokens used by the TUI (Standard §9).
//!
//! The palette **only** applies to curated content; the `loran help`
//! live-capture frame stays monochrome (Spec §2 decision #11). When
//! `NO_COLOR=1` is set the [`Palette::monochrome`] constructor returns
//! a token set that resolves to terminal defaults.

use ratatui::style::Color;

/// Void Navy — page / pane background. Standard §9.
pub const VOID_NAVY: Color = Color::Rgb(0x00, 0x00, 0x27);
/// Molten Amber — primary foreground (text, focused borders).
pub const MOLTEN_AMBER: Color = Color::Rgb(0xff, 0xb7, 0x00);
/// Steel Halo — secondary foreground (unfocused chrome).
pub const STEEL_HALO: Color = Color::Rgb(0xb0, 0xb8, 0xc4);
/// Forge Glow — accent (active row, callouts).
pub const FORGE_GLOW: Color = Color::Rgb(0xff, 0x6a, 0x00);
/// Slate Iron — subdued background (sidebar, modal scrim).
pub const SLATE_IRON: Color = Color::Rgb(0x1f, 0x29, 0x37);

/// Token set the TUI reads when colouring chrome and content. The
/// [`Palette::full`] constructor returns the Steelbore palette;
/// [`Palette::monochrome`] returns terminal-default values for the
/// `NO_COLOR=1` cascade.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub subdued_bg: Color,
}

impl Palette {
    /// Steelbore palette (Standard §9).
    #[must_use]
    pub const fn full() -> Self {
        Self {
            background: VOID_NAVY,
            foreground: MOLTEN_AMBER,
            muted: STEEL_HALO,
            accent: FORGE_GLOW,
            subdued_bg: SLATE_IRON,
        }
    }

    /// Monochrome fallback when `NO_COLOR=1` is set: every token
    /// resolves to [`Color::Reset`] so the terminal uses its default
    /// foreground and background.
    #[must_use]
    pub const fn monochrome() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::Reset,
            muted: Color::Reset,
            accent: Color::Reset,
            subdued_bg: Color::Reset,
        }
    }

    /// Pick a palette based on the `NO_COLOR` env var, honouring the
    /// CLI's `--no-color` cascade.
    #[must_use]
    pub fn from_env() -> Self {
        if std::env::var_os("NO_COLOR").is_some() {
            Self::monochrome()
        } else {
            Self::full()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Palette;
    use ratatui::style::Color;

    #[test]
    fn full_palette_uses_steelbore_tokens() {
        let p = Palette::full();
        assert_eq!(p.background, super::VOID_NAVY);
        assert_eq!(p.foreground, super::MOLTEN_AMBER);
    }

    #[test]
    fn monochrome_palette_is_all_reset() {
        let p = Palette::monochrome();
        for token in [p.background, p.foreground, p.muted, p.accent, p.subdued_bg] {
            assert_eq!(token, Color::Reset);
        }
    }
}
