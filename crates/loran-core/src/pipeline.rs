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

use crate::extract::{ExtractError, extract_tarball, extract_zip};
use crate::fetch::{FetchClient, FetchError, FetchOutcome};
use crate::meta::{MetaError, SourceMetaStore};
use crate::signing::{SignError, verify_any};

/// Canonical name for the upstream Steelbore pages source. Used as the
/// key in `sources.toml` and the JSON `data.source` field.
pub const SOURCE_UPSTREAM_PAGES: &str = "upstream-pages";

/// Canonical name for the tldr-pages mirror.
pub const SOURCE_TLDR_PAGES: &str = "tldr-pages";

/// tldr-pages archive URL (Spec §11). The archive is unsigned — no
/// minisign step.
pub const TLDR_PAGES_URL: &str = "https://tldr-pages.github.io/assets/tldr.zip";

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
#[derive(Debug, Clone, PartialEq, Eq, schemars::JsonSchema, serde::Serialize)]
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
    /// Override the trust-root public-key set (raw base64 minisign
    /// form). Verification accepts a signature that matches **any**
    /// key in this slice — the parallel-key transition primitive
    /// documented in `OPERATIONS.md`. A length-one set behaves
    /// identically to single-key verification.
    pub public_keys: Vec<String>,
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
    ///
    /// Each URL and the public key honour an environment override so
    /// integration tests can point the pipeline at a staging publisher
    /// (or an unreachable loopback address) without modifying source:
    ///
    /// - `LORAN_PAGES_MANIFEST_URL`
    /// - `LORAN_PAGES_TARBALL_URL`
    /// - `LORAN_PAGES_SIG_URL`
    /// - `LORAN_PAGES_PUBLIC_KEY` (comma-separated for parallel-key
    ///   transitions; single value is the common case)
    #[must_use]
    pub fn default_publisher(target_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            manifest_url: env_or("LORAN_PAGES_MANIFEST_URL", PUBLISHER_PAGES_MANIFEST_URL),
            tarball_url: env_or("LORAN_PAGES_TARBALL_URL", PUBLISHER_PAGES_TARBALL_URL),
            signature_url: env_or("LORAN_PAGES_SIG_URL", PUBLISHER_PAGES_SIG_URL),
            public_keys: env_or_keys("LORAN_PAGES_PUBLIC_KEY", PUBLISHER_PUBLIC_KEY),
            target_dir: target_dir.into(),
            source_name: SOURCE_UPSTREAM_PAGES.to_owned(),
            dry_run: false,
            force_refresh: false,
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Parse `KEY` as a comma-separated list of minisign public keys.
/// Falls back to a single-element vec containing `default` when unset
/// or empty. Whitespace and empty entries are trimmed.
fn env_or_keys(key: &str, default: &str) -> Vec<String> {
    let raw = std::env::var(key).unwrap_or_default();
    if raw.trim().is_empty() {
        return vec![default.to_owned()];
    }
    raw.split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
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

    let key_refs: Vec<&str> = opts.public_keys.iter().map(String::as_str).collect();
    verify_any(&tarball, &sig_text, &key_refs)?;

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

/// Canonical tldr-pages extracted-tree target:
/// `$XDG_CACHE_HOME/loran/tldr/extracted/` (Spec §5).
#[must_use]
pub fn default_tldr_target() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|d| d.join("loran").join("tldr").join("extracted"))
}

#[allow(dead_code)] // helper for callers wiring sub-phase 2B+
fn _data_dir_unused(_p: &Path) {}

/// Caller-supplied knobs for [`update_tldr`].
///
/// Unlike [`UpdateOpts`], this carries no `signature_url` or
/// `public_key` because tldr-pages doesn't sign its archive (Spec §11).
/// The pipeline still applies the body-size limit from
/// [`crate::FetchClient`] and the path-traversal protection inside
/// [`extract_zip`].
#[derive(Debug, Clone)]
pub struct UpdateTldrOpts {
    /// Override the zip URL.
    pub archive_url: String,
    /// Where to install the extracted tree. Typically
    /// `$XDG_CACHE_HOME/loran/tldr/extracted/`.
    pub target_dir: std::path::PathBuf,
    /// Source name for the meta-store record. Defaults to
    /// `"tldr-pages"`.
    pub source_name: String,
    /// Plan but don't install.
    pub dry_run: bool,
    /// Ignore the cached `ETag` and re-fetch unconditionally.
    pub force_refresh: bool,
}

impl UpdateTldrOpts {
    /// Default options. `LORAN_TLDR_ARCHIVE_URL` overrides the zip URL
    /// so integration tests can run against an unreachable loopback.
    #[must_use]
    pub fn default_publisher(target_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            archive_url: env_or("LORAN_TLDR_ARCHIVE_URL", TLDR_PAGES_URL),
            target_dir: target_dir.into(),
            source_name: SOURCE_TLDR_PAGES.to_owned(),
            dry_run: false,
            force_refresh: false,
        }
    }
}

/// Refresh the tldr-pages mirror.
///
/// Pipeline (no minisign step — tldr-pages doesn't sign):
///
/// 1. Read the previous `ETag` from `meta_store` for `tldr-pages`.
/// 2. `client.fetch_bytes(archive_url, prev_etag)` — short-circuits to
///    [`UpdateOutcome::NotModified`] on 304.
/// 3. `extract_zip(bytes, target_dir)` — atomic install with path-
///    traversal protection.
/// 4. Update `meta_store` with the new `ETag` + `fetched_at`. (No
///    version: tldr-pages doesn't ship a manifest.)
///
/// On `opts.dry_run = true`, steps 1–2 run and the call returns
/// [`UpdateOutcome::DryRun`] with an empty version field.
pub fn update_tldr(
    client: &FetchClient,
    meta_store: &SourceMetaStore,
    opts: &UpdateTldrOpts,
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

    let bytes_outcome = client.fetch_bytes(&opts.archive_url, prev_etag)?;
    let bytes = match bytes_outcome {
        FetchOutcome::NotModified => return Ok(UpdateOutcome::NotModified),
        FetchOutcome::Fresh(b) => b,
    };

    // tldr-pages doesn't publish a version. Use a fingerprint of the
    // archive bytes as a stand-in so the meta-store record can
    // distinguish refreshes.
    let synthetic_version = format!("sha256:{}", crate::sha256_hex(&bytes));

    if opts.dry_run {
        return Ok(UpdateOutcome::DryRun {
            version: synthetic_version,
            etag: String::new(), // no ETag inspection without a real fetch this round
        });
    }

    extract_zip(&bytes, &opts.target_dir)?;

    let now = jiff::Timestamp::now();
    let source_name = opts.source_name.clone();
    let synth = synthetic_version.clone();
    meta_store.update_source(&source_name, move |m| {
        // We don't surface the server's ETag because fetch_bytes doesn't
        // return headers in this version; the synthetic version field
        // doubles as the cache invalidator.
        m.etag = Some(synth.clone());
        m.version = Some(synth);
        m.fetched_at = Some(now);
    })?;

    Ok(UpdateOutcome::Updated {
        version: synthetic_version,
        etag: String::new(),
    })
}

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
        assert_eq!(opts.public_keys, vec![PUBLISHER_PUBLIC_KEY.to_owned()]);
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
