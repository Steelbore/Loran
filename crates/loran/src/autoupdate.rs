// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Opt-in catalog auto-update.
//!
//! When the user has enabled `[update] auto_update` (see
//! [`crate::config`]), the catalog read verbs call [`maybe_refresh`]
//! before building the index. If the cached upstream catalog is older
//! than the configured interval, Loran performs one best-effort
//! `loran update`-equivalent refresh of the upstream pages, reusing the
//! stored `ETag` so an unchanged catalog costs a single 304 round trip.
//! The freshly extracted tree is then picked up by
//! [`crate::index_loader`] for the in-flight command.
//!
//! The refresh is **never fatal**: any failure (offline, publisher
//! down, signature mismatch) is logged at `debug` and the command
//! proceeds against the cached/bundled catalog. `--offline` (or a
//! non-empty `LORAN_OFFLINE`) suppresses it entirely.

use std::time::Duration;

use loran_core::{
    FetchClient, SOURCE_UPSTREAM_PAGES, SourceMetaStore, UpdateOpts, UpdateOutcome,
    default_pages_target, update_pages,
};

use crate::cli::Cli;
use crate::config::Config;

/// Refresh the upstream catalog if auto-update is enabled and the cache
/// is stale. Silent on the happy path; best-effort and non-fatal.
pub(crate) fn maybe_refresh(cli: &Cli) {
    if is_offline(cli) {
        return;
    }
    let config = Config::load();
    if !config.update.auto_update {
        return;
    }

    let Ok(store) = SourceMetaStore::with_default_path() else {
        return;
    };
    if !is_stale(&store, config.update.interval) {
        return;
    }
    let Some(target) = default_pages_target() else {
        return;
    };

    let client = FetchClient::new();
    let opts = UpdateOpts::default_publisher(target);
    match update_pages(&client, &store, &opts) {
        Ok(UpdateOutcome::Updated { version, .. }) => {
            if !cli.global.quiet {
                eprintln!("loran: catalog auto-updated to {version}");
            }
        }
        // Already current (NotModified) — or a DryRun we never request
        // here. Nothing to report.
        Ok(_) => {}
        Err(err) => {
            // Background best-effort: stay quiet on stderr and leave a
            // trace for `-v`. The command continues against the cached
            // catalog.
            tracing::debug!(error = %err, "catalog auto-update failed; using cached catalog");
        }
    }
}

/// Whether network access is suppressed for this invocation.
fn is_offline(cli: &Cli) -> bool {
    cli.global.offline
        || std::env::var("LORAN_OFFLINE")
            .ok()
            .is_some_and(|v| !v.is_empty() && v != "0")
}

/// Stale when the upstream catalog has never been fetched, or its last
/// fetch is older than `interval`.
fn is_stale(store: &SourceMetaStore, interval: Duration) -> bool {
    let Ok(file) = store.load() else {
        // Can't read the meta file — don't trigger network churn on
        // every command; treat as fresh and let an explicit
        // `loran update` recover.
        return false;
    };
    let Some(fetched_at) = file
        .sources
        .get(SOURCE_UPSTREAM_PAGES)
        .and_then(|m| m.fetched_at)
    else {
        // Never fetched: only the bundled core is in play, so a first
        // refresh is worthwhile.
        return true;
    };
    let now = jiff::Timestamp::now();
    let age_secs = now.as_second().saturating_sub(fetched_at.as_second());
    let interval_secs = i64::try_from(interval.as_secs()).unwrap_or(i64::MAX);
    age_secs >= interval_secs
}
