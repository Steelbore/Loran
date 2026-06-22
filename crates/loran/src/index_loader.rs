// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Shared index builder for the read-side verbs (`show`, `list`,
//! `find`, `search`, `categories`).
//!
//! Combines the following precedence layers per Spec §5.1 (lowest to
//! highest):
//!
//! 1. Bundled upstream pages — compiled into the binary via
//!    [`BundledPagesIngestor`]. The always-available offline core.
//! 2. `$XDG_DATA_HOME/loran/pages/` — the downloaded upstream catalog
//!    written by `loran update`. Overrides bundled pages by name and
//!    contributes any pages the offline core doesn't carry. Absent
//!    until the first successful update.
//! 3. `$XDG_DATA_HOME/loran/overlays/<distro>/` — distro overlay,
//!    resolved from `/etc/os-release` or the `LORAN_DISTRO_OVERRIDE`
//!    env var.
//! 4. `$XDG_DATA_HOME/loran/overlays/user/` — user overlay.
//!
//! Every on-disk layer is skipped silently when its directory doesn't
//! exist (fresh install, no update yet, no user overlay yet, …).
//!
//! Optionally augmented at the lowest precedence by auto-synthesised
//! Spacecraft Software-CLI pages from [`DescribeIngestor`] when the
//! `LORAN_DESCRIBE_BINARIES` env var is set (WP-P3.04). Curated
//! pages always overlay on top.

use loran_core::BundledPagesIngestor;
use loran_index::{
    DescribeIngestor, Index, Ingestor, LayeredIngestor, MarkdownPagesIngestor, OverlayLayer,
    detect_distro_id,
};

/// Build the merged read-side index, optionally overriding the
/// active distro overlay name.
///
/// Precedence for the distro layer name (highest first):
/// 1. `overlay_override` — the `--overlay <NAME>` CLI flag.
/// 2. `LORAN_DISTRO_OVERRIDE` env var.
/// 3. `/etc/os-release` `ID=` line.
/// 4. `"generic"` fallback.
pub(crate) fn build_layered_index_with_overlay(
    overlay_override: Option<&str>,
) -> Result<Index, String> {
    // Auto-synthesised pages (Phase 3 `DescribeIngestor`) sit
    // *underneath* the bundled curated catalog: curated entries
    // always overlay on top, so the synthesis is purely a fallback
    // for tools that haven't been hand-written yet.
    let mut base: Vec<loran_pages::Page> = Vec::new();
    if let Some(describer) = DescribeIngestor::from_env() {
        let synth = describer
            .ingest()
            .map_err(|e| format!("describe ingest failed: {e}"))?;
        base.extend(synth);
    }

    let bundled = BundledPagesIngestor::new()
        .ingest()
        .map_err(|e| format!("bundled-pages ingest failed: {e}"))?;

    // Curated pages override anything DescribeIngestor synthesised —
    // we collapse by name here so the resulting Vec is duplicate-free
    // before LayeredIngestor's `with_base_pages` accepts it.
    let mut by_name: std::collections::HashMap<String, loran_pages::Page> =
        base.into_iter().map(|p| (p.name.clone(), p)).collect();
    for page in bundled {
        by_name.insert(page.name.clone(), page);
    }

    // The downloaded upstream catalog (`loran update` extracts it to
    // `$XDG_DATA_HOME/loran/pages/`) overrides the compiled-in bundled
    // pages by name and supplies any the offline core lacks. This is
    // what closes the update→read loop: without it, `loran update`
    // would refresh a tree nothing ever reads. Skipped until the first
    // successful update creates the directory.
    if let Some(data_dir) = loran_core::data_home() {
        let pages_dir = data_dir.join("loran").join("pages");
        if pages_dir.is_dir() {
            let downloaded = MarkdownPagesIngestor::new(&pages_dir)
                .ingest()
                .map_err(|e| format!("upstream pages ingest failed: {e}"))?;
            for page in downloaded {
                by_name.insert(page.name.clone(), page);
            }
        }
    }

    let merged_base: Vec<loran_pages::Page> = by_name.into_values().collect();

    let layers = on_disk_overlay_layers(overlay_override);
    let ingestor = LayeredIngestor::with_base_pages(merged_base, layers);
    let pages = ingestor
        .ingest()
        .map_err(|e| format!("overlay merge failed: {e}"))?;

    Index::build(pages).map_err(|e| format!("index build failed: {e}"))
}

/// Resolve the on-disk overlay layers (distro first, then user) under
/// `$XDG_DATA_HOME/loran/overlays/`. Returns an empty vec when no
/// data dir exists on this platform.
fn on_disk_overlay_layers(overlay_override: Option<&str>) -> Vec<OverlayLayer> {
    let Some(data_dir) = loran_core::data_home() else {
        return Vec::new();
    };
    let overlays = data_dir.join("loran").join("overlays");
    let distro = active_distro(overlay_override);
    vec![
        OverlayLayer::new("distro", overlays.join(&distro)),
        OverlayLayer::new("user", overlays.join("user")),
    ]
}

/// Active-distro overlay name. Precedence:
/// 1. `overlay_override` — the `--overlay <NAME>` CLI flag.
/// 2. `LORAN_DISTRO_OVERRIDE` env var (used by tests).
/// 3. `/etc/os-release` `ID=` line.
/// 4. `"generic"` fallback.
fn active_distro(overlay_override: Option<&str>) -> String {
    if let Some(name) = overlay_override {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_owned();
        }
    }
    if let Ok(override_id) = std::env::var("LORAN_DISTRO_OVERRIDE") {
        if !override_id.is_empty() {
            return override_id;
        }
    }
    detect_distro_id()
}
