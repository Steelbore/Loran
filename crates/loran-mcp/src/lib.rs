// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![deny(unsafe_code)]

//! Loran MCP — read-only Model Context Protocol server.
//!
//! Phase 0 placeholder. The MCP surface lands in Phase 3 (Bloom) per
//! WP-P3.02. The surface is strictly read-only by design — only `list`,
//! `show`, `find`, `search`, and `categories` are exposed. Write verbs
//! (`update`, `new`, `validate`) and the subprocess-spawning `help` verb
//! are deliberately excluded (Spec §12.2).
//!
//! This is the one crate where `tokio` is permitted; everywhere else,
//! Loran is synchronous.

/// Placeholder used to verify the workspace builds cleanly during Phase 0.
pub const fn placeholder() {}
