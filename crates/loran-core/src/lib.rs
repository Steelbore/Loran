// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![forbid(unsafe_code)]

//! Loran core — orchestration and resolution chains.
//!
//! Phase 1B surface: [`BundledPagesIngestor`], the read-side source
//! over the build-time page bundle. Resolution chains (`resolve_show`,
//! `resolve_find`, `resolve_search`) and the live `--help` capture
//! engine land in Sub-phases 1C–1D per `loran-plan-v0_1.md` WP-P1.04
//! and WP-P1.05.

mod bundled;

pub use bundled::BundledPagesIngestor;
