// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Shared index builder for the read-side verbs (`show`, `list`,
//! `find`, `search`, `categories`).
//!
//! Combines three precedence layers per Spec §5.1:
//!
//! 1. Bundled upstream pages — compiled into the binary via
//!    [`BundledPagesIngestor`].
//! 2. `$XDG_DATA_HOME/loran/overlays/<distro>/` — distro overlay,
//!    resolved from `/etc/os-release` or the `LORAN_DISTRO_OVERRIDE`
//!    env var.
//! 3. `$XDG_DATA_HOME/loran/overlays/user/` — user overlay.
//!
//! Either overlay root is skipped silently when its directory doesn't
//! exist (fresh install, no user overlay yet, …).
//!
//! Optionally augmented at the lowest precedence by auto-synthesised
//! Steelbore-CLI pages from [`DescribeIngestor`] when the
//! `LORAN_DESCRIBE_BINARIES` env var is set (WP-P3.04). Curated
//! pages always overlay on top.

use loran_core::BundledPagesIngestor;
use loran_index::{
    DescribeIngestor, Index, Ingestor, LayeredIngestor, OverlayLayer, detect_distro_id,
};

/// Build the merged read-side index.
///
/// Returns a string error so the caller (every read-verb handler) can
/// emit a `INDEX_NOT_BUILT` envelope without further conversion.
pub(crate) fn build_layered_index() -> Result<Index, String> {
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
    let merged_base: Vec<loran_pages::Page> = by_name.into_values().collect();

    let layers = on_disk_overlay_layers();
    let ingestor = LayeredIngestor::with_base_pages(merged_base, layers);
    let pages = ingestor
        .ingest()
        .map_err(|e| format!("overlay merge failed: {e}"))?;

    Index::build(pages).map_err(|e| format!("index build failed: {e}"))
}

/// Resolve the on-disk overlay layers (distro first, then user) under
/// `$XDG_DATA_HOME/loran/overlays/`. Returns an empty vec when no
/// data dir exists on this platform.
fn on_disk_overlay_layers() -> Vec<OverlayLayer> {
    let Some(data_dir) = dirs::data_dir() else {
        return Vec::new();
    };
    let overlays = data_dir.join("loran").join("overlays");
    let distro = active_distro();
    vec![
        OverlayLayer::new("distro", overlays.join(&distro)),
        OverlayLayer::new("user", overlays.join("user")),
    ]
}

/// Active-distro overlay name. Honours `LORAN_DISTRO_OVERRIDE` so
/// tests can pin a specific layer without writing to
/// `/etc/os-release`.
fn active_distro() -> String {
    if let Ok(override_id) = std::env::var("LORAN_DISTRO_OVERRIDE") {
        if !override_id.is_empty() {
            return override_id;
        }
    }
    detect_distro_id()
}
