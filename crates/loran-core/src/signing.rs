// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Minisign signature verification for upstream tarballs.
//!
//! Spec §2 decision #6 + §11: every upstream pages tarball is signed
//! with [minisign](https://jedisct1.github.io/minisign/) — ed25519,
//! single-round, no PKI. The publisher's public key is **baked into
//! the Loran binary at compile time** via `include_str!`. Key rotation
//! requires a new Loran release; older binaries fetching tarballs
//! signed by a rotated key will fail with [`SignError::Mismatch`] and
//! exit with `TARBALL_VERIFY_FAILED = 11`.
//!
//! ## Trust model
//!
//! - **Single trust root.** Exactly one minisign public key per Loran
//!   release. No keyring, no SHA-256 trust-on-first-use, no CT.
//! - **Hard failure.** A verify failure is *never* recoverable inside
//!   the same `loran update` invocation. The caller must surface
//!   `TARBALL_VERIFY_FAILED` to the user and refuse to extract.
//! - **Phase boundary.** This module is consumed by Sub-phase 2A's
//!   tarball pipeline (WP-P2.07 fetch + WP-P2.08 extract +
//!   WP-P2.10 publisher pipeline). The publisher's real public key
//!   ships when WP-P2.10 picks an upstream CDN; until then, the
//!   `verify` function works against any caller-supplied public key.

use minisign_verify::{PublicKey, Signature};
use thiserror::Error;

/// Errors raised by [`verify`].
#[derive(Debug, Error)]
pub enum SignError {
    /// The supplied public-key string is not a valid minisign public key.
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    /// The supplied signature string is not a valid minisign signature.
    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    /// The signature did not verify against the payload + public key.
    /// **This is the hard-failure case** — never fall through, never
    /// extract.
    #[error("signature did not match payload (cryptographic verify failed)")]
    Mismatch,
}

/// Verify a detached minisign signature.
///
/// `payload` is the bytes the signature covers (e.g. the gzipped
/// tarball). `signature_text` is the contents of the `.minisig` file.
/// `public_key_text` is the contents of the public-key file (the line
/// after the `untrusted comment:` header).
///
/// Returns `Ok(())` only if the signature cryptographically verifies
/// against `payload` under `public_key_text`. Any decode error or
/// signature mismatch surfaces as a distinct [`SignError`] variant.
pub fn verify(
    payload: &[u8],
    signature_text: &str,
    public_key_text: &str,
) -> Result<(), SignError> {
    let public_key = PublicKey::from_base64(extract_key_line(public_key_text))
        .map_err(|e| SignError::InvalidPublicKey(e.to_string()))?;
    let signature = Signature::decode(signature_text)
        .map_err(|e| SignError::InvalidSignature(e.to_string()))?;
    public_key
        .verify(payload, &signature, false)
        .map_err(|_| SignError::Mismatch)
}

/// Strip the `untrusted comment:` header line from a minisign public
/// key file and return the bare base64 key. If no such header is
/// present the input is passed through trimmed.
fn extract_key_line(public_key_text: &str) -> &str {
    public_key_text
        .lines()
        .find(|line| !line.is_empty() && !line.starts_with("untrusted comment:"))
        .unwrap_or(public_key_text.trim())
}

#[cfg(test)]
mod tests {
    use super::{SignError, verify};

    /// Real minisign test fixtures generated 2026-05-12 with the
    /// nixpkgs minisign 0.12 binary against the literal payload
    /// `"Hello, Loran.\n"`. **Test-only key — never used in production.**
    const TEST_PUBLIC_KEY: &str = "untrusted comment: minisign public key 2F5D3C9410A6BE2C
RWQsvqYQlDxdL2X0KKUsxVNyWw9P0tBXOVJzTI0sD845q4PE5zlISFHM";

    const TEST_SIGNATURE: &str = "untrusted comment: signature from minisign secret key
RUQsvqYQlDxdLwQcgdWGwFuKRBzDuOA2RXsED8cnOWnyFZmyvqXSdGc9uEIwzmK25SKG/VLDgNFYE6bbNqu36XepwtYY1pIGogQ=
trusted comment: Loran test signature, do not use in production.
AXRNotITPOsPYT+ehfb8hOek42NaoeecflDu0TgLrkT6MDQ9Eh4rHKj4LU+AxBb3QrrE2MSU5w57bEb6Xe0IAg==";

    const TEST_PAYLOAD: &[u8] = b"Hello, Loran.\n";

    #[test]
    fn valid_signature_verifies() {
        verify(TEST_PAYLOAD, TEST_SIGNATURE, TEST_PUBLIC_KEY)
            .expect("real minisign signature must verify");
    }

    #[test]
    fn payload_mutation_makes_signature_invalid() {
        let mutated: Vec<u8> = b"Hello, Loran?\n".to_vec();
        let err = verify(&mutated, TEST_SIGNATURE, TEST_PUBLIC_KEY).unwrap_err();
        assert!(matches!(err, SignError::Mismatch), "got {err:?}");
    }

    #[test]
    fn invalid_public_key_returns_typed_error() {
        let err = verify(TEST_PAYLOAD, TEST_SIGNATURE, "not a valid public key").unwrap_err();
        assert!(matches!(err, SignError::InvalidPublicKey(_)), "got {err:?}");
    }

    #[test]
    fn invalid_signature_returns_typed_error() {
        let err = verify(
            TEST_PAYLOAD,
            "completely bogus signature data",
            TEST_PUBLIC_KEY,
        )
        .unwrap_err();
        assert!(matches!(err, SignError::InvalidSignature(_)), "got {err:?}");
    }

    #[test]
    fn signature_against_different_payload_fails_mismatch_not_decode() {
        let different = b"a completely different payload that won't verify".to_vec();
        let err = verify(&different, TEST_SIGNATURE, TEST_PUBLIC_KEY).unwrap_err();
        assert!(
            matches!(err, SignError::Mismatch),
            "wrong-payload must surface as Mismatch, not a decode error; got {err:?}"
        );
    }

    #[test]
    fn extract_key_line_handles_bare_key() {
        let bare = "RWQsvqYQlDxdL2X0KKUsxVNyWw9P0tBXOVJzTI0sD845q4PE5zlISFHM";
        assert_eq!(super::extract_key_line(bare), bare);
    }

    #[test]
    fn extract_key_line_skips_untrusted_comment() {
        let with_comment = "untrusted comment: minisign public key XYZ\nABCDEF";
        assert_eq!(super::extract_key_line(with_comment), "ABCDEF");
    }
}
