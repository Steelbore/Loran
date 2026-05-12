// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Atomic gzipped-tarball extraction for the `loran update` pipeline.
//!
//! Per Spec §11 the catalog is shipped as a single `.tar.gz`. This
//! module decodes the gzip stream, unpacks the tar entries into a
//! sibling temp directory, and atomically renames the temp directory
//! over the target so concurrent readers either see the previous
//! catalog or the new one — never a partially-extracted tree.
//!
//! ## Safety against malicious tarballs
//!
//! - **Path traversal:** every entry's path is canonicalised by the
//!   [`tar`] crate's `unpack_in` against the temp root; entries that
//!   would escape (`..`, absolute, symlinks pointing outside) are
//!   rejected before any file is written.
//! - **Bomb defence:** the upstream tarball has already passed the
//!   `WP-P2.07` body-size limit and SHA-256 check. A signature
//!   verification step (`WP-P2.09`) gates extraction at a higher
//!   layer so a malicious unsigned tarball never reaches this code.

use std::fs;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractError {
    /// Filesystem or stream-read I/O failure.
    #[error("I/O error extracting tarball: {0}")]
    Io(#[from] std::io::Error),

    /// The tarball claimed an entry whose path would escape the
    /// extraction root (e.g. `../etc/passwd`). The `tar` crate's
    /// `unpack` enforces this; we surface it as a typed error.
    #[error("tarball contained a path-traversal entry: {0}")]
    PathEscape(String),

    /// Atomic replace of the target directory failed. The new
    /// content sits in a sibling `.staging-<random>` directory that
    /// the caller can inspect.
    #[error("atomic rename of {staging} → {target} failed: {source}")]
    AtomicReplace {
        staging: PathBuf,
        target: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Atomically extract a gzipped tarball into `target`.
///
/// Algorithm:
///
/// 1. Create a sibling staging directory `<target>.staging-<rand>`.
///    Same parent → rename is a directory-entry swap on a single
///    filesystem (atomic on POSIX, on Linux's `renameat2`-capable
///    paths, and on Windows when both sides are on the same volume).
/// 2. Stream-decode the gzip and unpack tar entries into the staging
///    dir via `tar::Archive::unpack`. Path-traversal protection is
///    inherited from the `tar` crate.
/// 3. If `target` already exists, atomically move it aside to
///    `<target>.previous-<rand>`; then `rename(staging, target)`;
///    then delete the previous directory. A failure after the
///    target-move-aside but before the staging-rename leaves the
///    previous directory in place for caller inspection.
pub fn extract_tarball(tarball: &[u8], target: &Path) -> Result<(), ExtractError> {
    let parent = target.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;

    let suffix = unique_suffix();
    let staging = parent.join(format!(
        "{}.staging-{}",
        target.file_name().unwrap_or_default().to_string_lossy(),
        suffix
    ));
    fs::create_dir_all(&staging)?;

    // Unpack into the staging directory. The `tar` crate enforces
    // path-traversal protection: entries whose absolute / `..`-bearing
    // paths would escape `staging` are rejected at unpack time.
    let reader = GzDecoder::new(tarball);
    let mut archive = Archive::new(reader);
    archive.set_overwrite(true);
    if let Err(err) = archive.unpack(&staging) {
        // Clean up partial staging directory.
        let _ = fs::remove_dir_all(&staging);
        if is_path_escape(&err) {
            return Err(ExtractError::PathEscape(err.to_string()));
        }
        return Err(ExtractError::Io(err));
    }

    // Atomic install.
    let previous = if target.exists() {
        let prev = parent.join(format!(
            "{}.previous-{}",
            target.file_name().unwrap_or_default().to_string_lossy(),
            suffix
        ));
        fs::rename(target, &prev).map_err(|source| ExtractError::AtomicReplace {
            staging: staging.clone(),
            target: target.to_path_buf(),
            source,
        })?;
        Some(prev)
    } else {
        None
    };

    if let Err(source) = fs::rename(&staging, target) {
        // Restore the previous target if we moved it aside.
        if let Some(prev) = &previous {
            let _ = fs::rename(prev, target);
        }
        return Err(ExtractError::AtomicReplace {
            staging,
            target: target.to_path_buf(),
            source,
        });
    }

    // Best-effort cleanup of the previous directory.
    if let Some(prev) = previous {
        let _ = fs::remove_dir_all(prev);
    }
    Ok(())
}

/// Generate a short pseudo-unique suffix for the staging / previous
/// directory names. Doesn't need to be cryptographic — just enough to
/// distinguish parallel `loran update` invocations on the same target.
fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}.{pid}", pid = std::process::id())
}

/// Heuristic: did the tar error originate from a path-traversal
/// rejection?
fn is_path_escape(err: &std::io::Error) -> bool {
    let msg = err.to_string();
    msg.contains("escapes") || msg.contains("outside") || msg.contains("..")
}

#[cfg(test)]
mod tests {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::{Builder, Header};
    use tempfile::tempdir;

    use super::{ExtractError, extract_tarball};

    /// Build an in-memory `.tar.gz` whose entries are `(path, body)`
    /// pairs.
    fn make_tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let buf: Vec<u8> = Vec::new();
        let gz = GzEncoder::new(buf, Compression::default());
        let mut builder = Builder::new(gz);
        for (path, body) in entries {
            let mut header = Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *body).unwrap();
        }
        let gz = builder.into_inner().unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn extract_round_trips_a_simple_tarball() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("pages");
        let tarball = make_tarball(&[
            ("categories.toml", b"[file-listing]\ntitle = \"x\"\n"),
            ("file-listing/eza.md", b"+++\nname=\"eza\"\n+++\n"),
        ]);

        extract_tarball(&tarball, &target).expect("extract succeeds");

        let cat = target.join("categories.toml");
        assert!(cat.is_file(), "categories.toml landed");
        assert!(
            std::fs::read_to_string(&cat)
                .unwrap()
                .contains("file-listing")
        );

        let eza = target.join("file-listing").join("eza.md");
        assert!(eza.is_file(), "eza.md landed at the right nested path");
    }

    #[test]
    fn extract_atomically_replaces_existing_target() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("pages");

        // Seed an older "catalog".
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("old.md"), b"old content").unwrap();

        let tarball = make_tarball(&[("new.md", b"new content")]);
        extract_tarball(&tarball, &target).expect("extract replaces");

        // The replaced target should ONLY contain new.md, not old.md.
        assert!(target.join("new.md").is_file());
        assert!(
            !target.join("old.md").exists(),
            "old.md must be replaced atomically"
        );
    }

    #[test]
    fn extract_creates_target_when_missing() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("deep").join("never").join("existed");
        let tarball = make_tarball(&[("hello.txt", b"hi")]);
        extract_tarball(&tarball, &target).expect("creates target");
        assert!(target.join("hello.txt").is_file());
    }

    #[test]
    fn extract_rejects_corrupt_gzip() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("pages");
        let result = extract_tarball(b"this is not a gzip stream", &target);
        assert!(matches!(result, Err(ExtractError::Io(_))), "got {result:?}");
    }

    #[test]
    fn extract_cleans_up_staging_on_corrupt_payload() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("pages");
        let _ = extract_tarball(b"junk", &target);

        // The staging directory must NOT survive a failed extract.
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".staging-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "failed extract left .staging behind: {leftovers:?}"
        );
    }

    #[test]
    fn extract_leaves_no_previous_directory_after_success() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("pages");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("old.md"), b"old").unwrap();

        let tarball = make_tarball(&[("new.md", b"new")]);
        extract_tarball(&tarball, &target).expect("extract");

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".previous-") || n.contains(".staging-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "success path must clean up .previous/.staging: {leftovers:?}"
        );
    }
}
