// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Update-pipeline orchestrator: fetch → verify → extract → cache-bust.
//!
//! Composes the four Sub-phase 2A primitives:
//!
//! - [`crate::FetchClient`] (manifest + signed-tarball download)
//! - [`crate::verify_minisign`] (ed25519 signature)
//! - [`crate::extract_tarball`] (atomic gzip-tar install)
//! - [`crate::Cache`] (postcard index cache invalidation)
//!
//! into the high-level entry points consumed by the `loran update`
//! sub-command (Sub-phase 2B `WP-P2.12`).
//!
//! ## Trust + correctness contract
//!
//! Every refresh is **all-or-nothing**. Failure in any step aborts the
//! pipeline with a typed error variant and leaves the previously-
//! installed catalog untouched. There is **no** "extract anyway" path
//! — `--require-signatures` is the only flag that opts out of
//! signature checking, and it only applies to sources known not to
//! sign (tldr-pages; not the Steelbore upstream).
//!
//! ## Publisher trust root
//!
//! `PUBLISHER_PUBLIC_KEY` is **the** trust root for upstream tarballs.
//! Currently set to a development placeholder (see the constant's
//! doc-comment); the real publisher key lands when the upstream CDN
//! pipeline launches alongside Sub-phase 2D. Until then, callers who
//! want to exercise the pipeline against a staging publisher pass
//! their own key via the `public_key` parameter on [`update_pages`].

use std::path::Path;

use crate::extract::{ExtractError, extract_tarball};
use crate::fetch::{FetchClient, FetchError, FetchOutcome};
use crate::meta::{MetaError, SourceMetaStore};
use crate::signing::{SignError, verify};

/// Canonical name for the upstream Steelbore pages source. Used as the
/// key in `sources.toml` and the JSON `data.source` field.
pub const SOURCE_UPSTREAM_PAGES: &str = "upstream-pages";

/// Publisher manifest URL.
///
/// **Placeholder.** The real upstream CDN endpoint is selected when
/// the publisher pipeline (Sub-phase 2D) launches. Until then this URL
/// will fail a manifest fetch — that's by design, so a `loran update`
/// against an un-launched publisher fails loud rather than silently.
pub const PUBLISHER_PAGES_MANIFEST_URL: &str = "https://Loran.Steelbore.com/pages/v1/pages.json";

/// Publisher tarball URL. Placeholder; see [`PUBLISHER_PAGES_MANIFEST_URL`].
pub const PUBLISHER_PAGES_TARBALL_URL: &str = "https://Loran.Steelbore.com/pages/v1/pages.tar.gz";

/// Publisher detached signature URL (minisign `.minisig`).
/// Placeholder; see [`PUBLISHER_PAGES_MANIFEST_URL`].
pub const PUBLISHER_PAGES_SIG_URL: &str =
    "https://Loran.Steelbore.com/pages/v1/pages.tar.gz.minisig";

/// Publisher's minisign public key.
///
/// **Placeholder — development key, NOT for production use.** Real
/// upstream releases replace this with the publisher's actual public
/// key as part of the Sub-phase 2D launch. Until then this constant
/// is the same test key as `signing::tests::TEST_PUBLIC_KEY`, embedded
/// here so the orchestration code compiles and unit tests can exercise
/// the verify step.
///
/// Key rotation: a new Loran release ships a new value here. Older
/// binaries fetching tarballs signed by a rotated key fail with
/// [`UpdateError::Sign`]`(SignError::Mismatch)`.
pub const PUBLISHER_PUBLIC_KEY: &str = "RWQsvqYQlDxdL2X0KKUsxVNyWw9P0tBXOVJzTI0sD845q4PE5zlISFHM";

/// Outcome of a single source update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// Manifest 304'd — local install already matches upstream.
    NotModified,
    /// Catalog was refreshed. Fields surface what changed.
    Updated { version: String, etag: String },
    /// `--dry-run` requested; manifest was fetched and reported but
    /// nothing was downloaded or extracted.
    DryRun { version: String, etag: String },
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("manifest / tarball fetch failed: {0}")]
    Fetch(#[from] FetchError),

    #[error("signature verification failed: {0}")]
    Sign(#[from] SignError),

    #[error("atomic extraction failed: {0}")]
    Extract(#[from] ExtractError),

    #[error("meta-store I/O failed: {0}")]
    Meta(#[from] MetaError),

    #[error("publisher pipeline returned no tarball body (got NotModified after manifest changed)")]
    UnexpectedNotModified,
}

/// Caller-supplied knobs for [`update_pages`].
#[derive(Debug, Clone)]
pub struct UpdateOpts {
    /// Override the manifest URL.
    pub manifest_url: String,
    /// Override the tarball URL.
    pub tarball_url: String,
    /// Override the signature URL.
    pub signature_url: String,
    /// Override the trust-root public key (raw base64 minisign form).
    pub public_key: String,
    /// Where to install the extracted catalog. Typically
    /// `$XDG_DATA_HOME/loran/pages/`.
    pub target_dir: std::path::PathBuf,
    /// Source name for the meta-store record. Defaults to
    /// `"upstream-pages"`.
    pub source_name: String,
    /// Plan but don't install.
    pub dry_run: bool,
    /// Ignore the cached `ETag` and re-fetch unconditionally.
    pub force_refresh: bool,
}

impl UpdateOpts {
    /// Default `UpdateOpts` targeting the placeholder Steelbore
    /// publisher. Caller must override `target_dir`.
    #[must_use]
    pub fn default_publisher(target_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            manifest_url: PUBLISHER_PAGES_MANIFEST_URL.to_owned(),
            tarball_url: PUBLISHER_PAGES_TARBALL_URL.to_owned(),
            signature_url: PUBLISHER_PAGES_SIG_URL.to_owned(),
            public_key: PUBLISHER_PUBLIC_KEY.to_owned(),
            target_dir: target_dir.into(),
            source_name: SOURCE_UPSTREAM_PAGES.to_owned(),
            dry_run: false,
            force_refresh: false,
        }
    }
}

/// Refresh the upstream Steelbore pages catalog.
///
/// Pipeline (all-or-nothing):
///
/// 1. Read the previous `ETag` from `meta_store` for this source.
/// 2. `client.fetch_manifest(manifest_url, prev_etag)` — short-circuit
///    to [`UpdateOutcome::NotModified`] on 304.
/// 3. `client.fetch_tarball(tarball_url, &manifest, None)` — verifies
///    SHA-256 internally.
/// 4. `client.fetch_bytes(signature_url, None)` — the `.minisig`.
/// 5. `verify(tarball, sig, public_key)` — fail-stop on
///    [`SignError::Mismatch`]; surfaces as
///    [`UpdateError::Sign`]`(SignError::Mismatch)`.
/// 6. `extract_tarball(tarball, target_dir)` — atomic install.
/// 7. Update `meta_store` with the new `ETag` + version + `fetched_at`.
///
/// On `opts.dry_run = true`, steps 1–2 run; subsequent steps are
/// skipped and the call returns [`UpdateOutcome::DryRun`].
pub fn update_pages(
    client: &FetchClient,
    meta_store: &SourceMetaStore,
    opts: &UpdateOpts,
) -> Result<UpdateOutcome, UpdateError> {
    let meta_file = meta_store.load()?;
    let prev_etag = if opts.force_refresh {
        None
    } else {
        meta_file
            .sources
            .get(&opts.source_name)
            .and_then(|m| m.etag.as_deref())
    };

    let manifest = match client.fetch_manifest(&opts.manifest_url, prev_etag)? {
        FetchOutcome::NotModified => return Ok(UpdateOutcome::NotModified),
        FetchOutcome::Fresh(m) => m,
    };

    if opts.dry_run {
        return Ok(UpdateOutcome::DryRun {
            version: manifest.version.clone(),
            etag: manifest.etag.clone(),
        });
    }

    let tarball_outcome = client.fetch_tarball(&opts.tarball_url, &manifest, None)?;
    let tarball = match tarball_outcome {
        FetchOutcome::Fresh(bytes) => bytes,
        FetchOutcome::NotModified => return Err(UpdateError::UnexpectedNotModified),
    };

    let sig_outcome = client.fetch_bytes(&opts.signature_url, None)?;
    let sig_bytes = match sig_outcome {
        FetchOutcome::Fresh(bytes) => bytes,
        FetchOutcome::NotModified => return Err(UpdateError::UnexpectedNotModified),
    };
    let sig_text = String::from_utf8_lossy(&sig_bytes).into_owned();

    verify(&tarball, &sig_text, &opts.public_key)?;

    extract_tarball(&tarball, &opts.target_dir)?;

    // Persist new meta record.
    let now = jiff::Timestamp::now();
    let version = manifest.version.clone();
    let new_etag = manifest.etag.clone();
    let source_name = opts.source_name.clone();
    let etag_for_meta = new_etag.clone();
    let version_for_meta = version.clone();
    meta_store.update_source(&source_name, move |m| {
        m.etag = Some(etag_for_meta);
        m.version = Some(version_for_meta);
        m.fetched_at = Some(now);
    })?;

    Ok(UpdateOutcome::Updated {
        version,
        etag: new_etag,
    })
}

/// Convenience wrapper for the canonical `$XDG_DATA_HOME/loran/pages/`
/// target. Mirrors the layout in `loran-spec-v0_2.md` §5.
#[must_use]
pub fn default_pages_target() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("loran").join("pages"))
}

#[allow(dead_code)] // helper for callers wiring sub-phase 2B+
fn _data_dir_unused(_p: &Path) {}

#[cfg(test)]
mod tests {
    use super::{PUBLISHER_PUBLIC_KEY, SOURCE_UPSTREAM_PAGES, UpdateOpts};

    #[test]
    fn default_publisher_opts_target_xdg_data_home() {
        let opts = UpdateOpts::default_publisher("/tmp/loran-test-pages");
        assert_eq!(opts.source_name, SOURCE_UPSTREAM_PAGES);
        assert!(opts.manifest_url.starts_with("https://"));
        assert!(opts.tarball_url.ends_with(".tar.gz"));
        assert!(opts.signature_url.ends_with(".minisig"));
        assert_eq!(opts.public_key, PUBLISHER_PUBLIC_KEY);
        assert!(!opts.dry_run);
        assert!(!opts.force_refresh);
    }

    #[test]
    fn publisher_key_decodes_as_a_minisign_public_key() {
        // Sanity: the baked-in placeholder must at least pass the
        // minisign decode step. A future swap of the constant to a
        // real key must keep this true.
        let result = minisign_verify::PublicKey::from_base64(PUBLISHER_PUBLIC_KEY);
        assert!(
            result.is_ok(),
            "PUBLISHER_PUBLIC_KEY must decode: {result:?}"
        );
    }
}
