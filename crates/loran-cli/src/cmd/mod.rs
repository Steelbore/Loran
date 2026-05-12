// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Per-sub-command handler modules.
//!
//! Each handler exposes a `run(cli: &Cli, args: &XArgs) -> ExitCode`
//! function called from `main::dispatch`. Handlers are responsible for
//! resolving their own output mode, building the index from the
//! bundled-pages ingestor, calling into `loran-core` for resolution,
//! and writing output through `JsonEmitter` (JSON) or stdout/stderr
//! directly (text).

pub(crate) mod categories;
pub(crate) mod describe;
pub(crate) mod find;
pub(crate) mod help;
pub(crate) mod list;
pub(crate) mod new;
pub(crate) mod schema;
pub(crate) mod search;
pub(crate) mod show;
pub(crate) mod update;
pub(crate) mod validate;
