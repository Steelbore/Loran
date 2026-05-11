// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Single inclusion site for the build-time-generated constants.
//!
//! `crates/loran-core/build.rs` writes
//! `$OUT_DIR/bundled_pages.rs` declaring `BUNDLED_PAGES` and
//! `BUNDLED_CATEGORIES`. Both `bundled.rs` and `categories.rs` import
//! through this module so the generated source is included exactly
//! once (otherwise each `include!` site would get its own copy and
//! clippy would flag the unused half).

include!(concat!(env!("OUT_DIR"), "/bundled_pages.rs"));
