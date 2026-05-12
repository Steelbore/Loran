// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![deny(unsafe_code)]

//! Loran MCP — read-only Model Context Protocol server.
//!
//! Implements the Model Context Protocol (2024-11-05 spec) over
//! newline-delimited JSON-RPC 2.0 on stdio. Five read-only verbs are
//! exposed (`list`, `show`, `find`, `search`, `categories`); write
//! verbs (`update`, `new`, `validate`) and the subprocess-spawning
//! `help` verb are deliberately excluded (Spec §12.2).
//!
//! Sync from end to end. There is no `tokio` — MCP stdio is a strict
//! request/response loop with one in-flight RPC at a time, so a blocking
//! read on stdin is correct and avoids dragging an async runtime into
//! the workspace.

mod handler;
mod protocol;
mod server;
mod tools;

pub use handler::Handler;
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, error_codes};
pub use server::{ServerError, serve_stdio};
pub use tools::ToolDefinition;
