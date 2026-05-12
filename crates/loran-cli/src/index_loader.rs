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

use loran_core::BundledPagesIngestor;
use loran_index::{Index, Ingestor, LayeredIngestor, OverlayLayer, detect_distro_id};

/// Build the merged read-side index.
///
/// Returns a string error so the caller (every read-verb handler) can
/// emit a `INDEX_NOT_BUILT` envelope without further conversion.
pub(crate) fn build_layered_index() -> Result<Index, String> {
    let bundled = BundledPagesIngestor::new()
        .ingest()
        .map_err(|e| format!("bundled-pages ingest failed: {e}"))?;

    let layers = on_disk_overlay_layers();

    let ingestor = LayeredIngestor::with_base_pages(bundled, layers);
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
