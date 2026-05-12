// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! HTTP manifest + tarball fetching for `loran update`.
//!
//! Per Spec §11 every refresh round trips through two HTTPS GETs:
//!
//! 1. `<pages.tar.gz>.json` — small manifest with `version`, `etag`,
//!    `sha256`, and `size`. Fetched with `If-None-Match` against the
//!    cached `ETag`, so an unchanged catalog short-circuits with a 304.
//! 2. `<pages.tar.gz>` — the actual gzipped tarball. Only fetched
//!    after the manifest says the `ETag` (or the `SHA-256`) has changed.
//!
//! TLS is via `rustls` (workspace-pinned `ureq` `default-features = false,
//! features = ["rustls"]`). No native-tls is allowed.
//!
//! ## Responsibilities
//!
//! - Manifest fetch + JSON decode.
//! - Tarball fetch with optional `If-None-Match` short-circuit.
//! - SHA-256 verification against the manifest before the bytes
//!   leave this module.
//!
//! Minisign signature verification and atomic extraction live in
//! sibling modules ([`crate::verify_minisign`], `WP-P2.08`).

use std::io::Read;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Upstream manifest shape (Spec §11).
///
/// Lives next to the tarball at `<pages.tar.gz>.json`. The publisher
/// pipeline (Phase 2 `WP-P2.10`) is responsible for emitting one
/// matching the schema below.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    /// Tarball version (e.g. `"v1.2.0"`). Caller compares against the
    /// cached version to decide whether a refresh is warranted.
    pub version: String,
    /// HTTP ``ETag`` header value associated with the tarball. Used as
    /// the `If-None-Match` value on the next fetch.
    pub etag: String,
    /// Lowercase hex SHA-256 of the tarball contents.
    pub sha256: String,
    /// Tarball size in bytes.
    pub size: u64,
}

#[derive(Debug, Error)]
pub enum FetchError {
    /// Non-2xx HTTP status (other than `304 Not Modified`, which is a
    /// success short-circuit and surfaces as `Ok(FetchOutcome::NotModified)`).
    #[error("HTTP error {status}: {url}")]
    Http { status: u16, url: String },

    /// Transport-level I/O failure (connection refused, DNS, TLS, …).
    #[error("transport error talking to {url}: {source}")]
    Transport {
        url: String,
        #[source]
        source: ureq::Error,
    },

    /// Manifest body did not decode as JSON.
    #[error("manifest at {url} is not valid JSON: {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    /// Tarball body's SHA-256 did not match the manifest. Surfaces
    /// up as exit code `TARBALL_VERIFY_FAILED = 11`.
    #[error("SHA-256 mismatch for tarball at {url} (expected {expected}, got {actual})")]
    Sha256Mismatch {
        url: String,
        expected: String,
        actual: String,
    },

    /// Body too large to safely buffer in memory. Defends against a
    /// malicious or misconfigured upstream serving a 10 GiB blob to a
    /// `loran update` command.
    #[error("body at {url} exceeded {limit_bytes}-byte read limit")]
    BodyTooLarge { url: String, limit_bytes: usize },

    /// Generic body-read I/O failure.
    #[error("I/O reading body from {url}: {source}")]
    Body {
        url: String,
        #[source]
        source: std::io::Error,
    },
}

/// Outcome of a conditional fetch.
#[derive(Debug, Clone)]
pub enum FetchOutcome<T> {
    /// Server responded with the requested body.
    Fresh(T),
    /// Server responded `304 Not Modified` — caller should keep
    /// using their existing cached copy.
    NotModified,
}

/// Maximum size for an in-memory body buffer. The upstream tarball is
/// expected to be in the low MiB range — a 32 MiB cap is well above
/// realistic content while still bounding worst-case memory.
pub const DEFAULT_BODY_LIMIT_BYTES: usize = 32 * 1024 * 1024;

/// Loran HTTP client wrapping a ureq agent + a body-size limit.
#[derive(Debug, Clone)]
pub struct FetchClient {
    agent: ureq::Agent,
    body_limit_bytes: usize,
}

impl Default for FetchClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FetchClient {
    /// Construct a client with the default ureq agent + body limit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::new_with_defaults(),
            body_limit_bytes: DEFAULT_BODY_LIMIT_BYTES,
        }
    }

    /// Override the in-memory body size limit. Returns `self` so the
    /// builder chains.
    #[must_use]
    pub fn with_body_limit(mut self, bytes: usize) -> Self {
        self.body_limit_bytes = bytes;
        self
    }

    /// Fetch a manifest, decode as JSON, return the parsed [`Manifest`].
    ///
    /// Honours `If-None-Match` when `if_none_match` is `Some`. Server
    /// `304` short-circuits to [`FetchOutcome::NotModified`].
    pub fn fetch_manifest(
        &self,
        url: &str,
        if_none_match: Option<&str>,
    ) -> Result<FetchOutcome<Manifest>, FetchError> {
        let bytes_outcome = self.fetch_bytes(url, if_none_match)?;
        match bytes_outcome {
            FetchOutcome::NotModified => Ok(FetchOutcome::NotModified),
            FetchOutcome::Fresh(bytes) => {
                let manifest = serde_json::from_slice::<Manifest>(&bytes).map_err(|source| {
                    FetchError::Decode {
                        url: url.to_owned(),
                        source,
                    }
                })?;
                Ok(FetchOutcome::Fresh(manifest))
            }
        }
    }

    /// Fetch a tarball, verify its SHA-256 against the manifest, and
    /// return the raw bytes. Pass `if_none_match` to short-circuit
    /// when the server's `ETag` matches a previously-cached body.
    pub fn fetch_tarball(
        &self,
        url: &str,
        manifest: &Manifest,
        if_none_match: Option<&str>,
    ) -> Result<FetchOutcome<Vec<u8>>, FetchError> {
        let bytes_outcome = self.fetch_bytes(url, if_none_match)?;
        match bytes_outcome {
            FetchOutcome::NotModified => Ok(FetchOutcome::NotModified),
            FetchOutcome::Fresh(bytes) => {
                let actual = sha256_hex(&bytes);
                if actual != manifest.sha256.to_lowercase() {
                    return Err(FetchError::Sha256Mismatch {
                        url: url.to_owned(),
                        expected: manifest.sha256.clone(),
                        actual,
                    });
                }
                Ok(FetchOutcome::Fresh(bytes))
            }
        }
    }

    /// Plain GET → bytes, with `If-None-Match` support. Returns
    /// [`FetchOutcome::NotModified`] on HTTP 304, [`FetchOutcome::Fresh`]
    /// otherwise. Bounded by [`Self::with_body_limit`].
    pub fn fetch_bytes(
        &self,
        url: &str,
        if_none_match: Option<&str>,
    ) -> Result<FetchOutcome<Vec<u8>>, FetchError> {
        let mut request = self.agent.get(url);
        if let Some(etag) = if_none_match {
            request = request.header("If-None-Match", etag);
        }

        let response = match request.call() {
            Ok(resp) => resp,
            Err(ureq::Error::StatusCode(304)) => {
                return Ok(FetchOutcome::NotModified);
            }
            Err(ureq::Error::StatusCode(code)) => {
                return Err(FetchError::Http {
                    status: code,
                    url: url.to_owned(),
                });
            }
            Err(source) => {
                return Err(FetchError::Transport {
                    url: url.to_owned(),
                    source,
                });
            }
        };

        let mut response = response;
        let mut reader = response.body_mut().as_reader();
        let mut buf = Vec::with_capacity(8 * 1024);
        // Use a limited reader so a hostile upstream can't OOM us.
        let mut limited = (&mut reader).take(self.body_limit_bytes as u64 + 1);
        limited
            .read_to_end(&mut buf)
            .map_err(|source| FetchError::Body {
                url: url.to_owned(),
                source,
            })?;

        if buf.len() > self.body_limit_bytes {
            return Err(FetchError::BodyTooLarge {
                url: url.to_owned(),
                limit_bytes: self.body_limit_bytes,
            });
        }
        Ok(FetchOutcome::Fresh(buf))
    }
}

/// Compute the lowercase hex SHA-256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{FetchError, FetchOutcome, Manifest, sha256_hex};

    #[test]
    fn sha256_of_known_bytes_matches_published_test_vector() {
        // Empty input → e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // "abc" → ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let manifest = Manifest {
            version: "v1.2.3".into(),
            etag: "\"abc-def\"".into(),
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            size: 1_234_567,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn manifest_decode_rejects_missing_required_fields() {
        let bad = r#"{"version": "v1", "etag": "x"}"#;
        assert!(serde_json::from_str::<Manifest>(bad).is_err());
    }

    #[test]
    fn fetch_outcome_variants_distinguish_fresh_from_not_modified() {
        let fresh: FetchOutcome<u32> = FetchOutcome::Fresh(7);
        let stale: FetchOutcome<u32> = FetchOutcome::NotModified;
        assert!(matches!(fresh, FetchOutcome::Fresh(7)));
        assert!(matches!(stale, FetchOutcome::NotModified));
    }

    #[test]
    fn sha256_mismatch_error_carries_both_hex_strings() {
        let err = FetchError::Sha256Mismatch {
            url: "https://example.test/x.tgz".into(),
            expected: "aaa".into(),
            actual: "bbb".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("aaa"));
        assert!(msg.contains("bbb"));
        assert!(msg.contains("https://example.test/x.tgz"));
    }
}
