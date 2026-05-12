// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Atomic archive extraction for the `loran update` pipeline.
//!
//! Two archive formats are supported:
//!
//! - **gzipped tarball** ([`extract_tarball`]) — the Steelbore upstream
//!   pages format per Spec §11. Decoded with `flate2::GzDecoder` →
//!   `tar::Archive`.
//! - **zip** ([`extract_zip`]) — the tldr-pages format per Spec §11.
//!   Decoded with the `zip` crate.
//!
//! Both formats share the staging + atomic-rename install logic in
//! [`with_staging`]: each call creates a sibling `<target>.staging-<rand>`
//! directory, runs the format-specific unpack into it, then atomically
//! renames the staging directory over the target. The previous target
//! (if any) is moved aside to `<target>.previous-<rand>` and removed
//! after the swap succeeds. A failure during unpack cleans up the
//! staging directory; a failure during rename restores the previous
//! target so the system never ends up with no catalog.
//!
//! ## Safety against malicious archives
//!
//! - **Path traversal:** every entry's path is canonicalised against
//!   the staging root. For tar, this is done by the `tar` crate's
//!   `unpack` directly. For zip, [`extract_zip`] computes the resolved
//!   path manually and rejects any entry whose normalised form would
//!   escape (`..`, absolute paths, drive letters, …).
//! - **Bomb defence:** upstream archives have already passed the
//!   `WP-P2.07` body-size limit and a SHA-256 check before reaching
//!   this layer; the signature verify step in `WP-P2.09` gates the
//!   pages tarball at a higher layer. The tldr archive carries no
//!   signature per Spec §11, but the body limit still applies.

use std::fs;
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};

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
    with_staging(target, |staging| {
        let reader = GzDecoder::new(tarball);
        let mut archive = Archive::new(reader);
        archive.set_overwrite(true);
        archive.unpack(staging).map_err(|err| {
            if is_path_escape(&err) {
                ExtractError::PathEscape(err.to_string())
            } else {
                ExtractError::Io(err)
            }
        })
    })
}

/// Atomically extract a zip archive into `target`.
///
/// Mirrors [`extract_tarball`]'s staging+rename semantics. Path-
/// traversal protection is hand-rolled (the `zip` crate doesn't enforce
/// it for us): every entry's resolved path must be a descendant of the
/// staging root, with no `..` parents or absolute roots in its
/// components.
pub fn extract_zip(zip_bytes: &[u8], target: &Path) -> Result<(), ExtractError> {
    with_staging(target, |staging| {
        let cursor = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| ExtractError::Io(std::io::Error::other(e.to_string())))?;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| ExtractError::Io(std::io::Error::other(e.to_string())))?;
            let Some(entry_path) = entry.enclosed_name() else {
                return Err(ExtractError::PathEscape(format!(
                    "zip entry name `{}` is not a safe relative path",
                    entry.name()
                )));
            };
            // Belt-and-braces re-check on the resolved path.
            if !is_within_staging(&entry_path) {
                return Err(ExtractError::PathEscape(format!(
                    "zip entry resolves outside staging: `{}`",
                    entry_path.display()
                )));
            }
            let dest = staging.join(&entry_path);
            if entry.is_dir() {
                fs::create_dir_all(&dest)?;
            } else {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut writer = fs::File::create(&dest)?;
                copy_zip_entry(&mut entry, &mut writer)?;
            }
        }
        Ok(())
    })
}

/// Copy a zip entry's body into `writer`. Wraps `std::io::copy` so the
/// caller can pass `&mut zip::read::ZipFile` without dealing with the
/// reader-type plumbing inline.
fn copy_zip_entry<R: Read, W: std::io::Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<(), ExtractError> {
    std::io::copy(reader, writer)
        .map(|_| ())
        .map_err(ExtractError::Io)
}

#[allow(dead_code)] // kept for potential future Seek-bearing zip backends
fn _seek_typed<R: Read + Seek>(_r: &mut R) {}

/// Reject `..`, absolute roots, and drive-letter prefixes in the
/// resolved relative path.
fn is_within_staging(path: &Path) -> bool {
    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    true
}

/// Common staging + atomic-install scaffolding. The closure is
/// responsible for the format-specific unpack and may return any
/// [`ExtractError`] variant — typically [`ExtractError::Io`] or
/// [`ExtractError::PathEscape`]. Errors are surfaced after the
/// staging directory is cleaned up.
fn with_staging<F>(target: &Path, unpack: F) -> Result<(), ExtractError>
where
    F: FnOnce(&Path) -> Result<(), ExtractError>,
{
    let parent = target.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;

    let suffix = unique_suffix();
    let staging = parent.join(format!(
        "{}.staging-{}",
        target.file_name().unwrap_or_default().to_string_lossy(),
        suffix
    ));
    fs::create_dir_all(&staging)?;

    if let Err(err) = unpack(&staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }

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
        if let Some(prev) = &previous {
            let _ = fs::rename(prev, target);
        }
        return Err(ExtractError::AtomicReplace {
            staging,
            target: target.to_path_buf(),
            source,
        });
    }

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
    use std::io::Write as _;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::{Builder, Header};
    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{ExtractError, extract_tarball, extract_zip};

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

    // ─── zip path ────────────────────────────────────────────────────

    /// Build an in-memory zip from `(path, body)` pairs.
    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let buf: Vec<u8> = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut writer = ZipWriter::new(cursor);
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (path, body) in entries {
            writer.start_file(*path, opts).unwrap();
            writer.write_all(body).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn extract_zip_round_trips_a_simple_archive() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("tldr");
        let archive = make_zip(&[
            ("pages/common/cat.md", b"# cat\n\nConcatenate.\n"),
            ("pages/linux/cat.md", b"# cat (linux)\n"),
        ]);

        extract_zip(&archive, &target).expect("extract zip");

        let common = target.join("pages").join("common").join("cat.md");
        assert!(common.is_file());
        assert!(
            std::fs::read_to_string(&common)
                .unwrap()
                .contains("Concatenate")
        );
        let linux = target.join("pages").join("linux").join("cat.md");
        assert!(linux.is_file());
    }

    #[test]
    fn extract_zip_cleans_up_staging_on_corrupt_payload() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("tldr");
        let _ = extract_zip(b"this is not a zip", &target);

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".staging-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "failed zip extract left .staging behind: {leftovers:?}"
        );
    }

    #[test]
    fn extract_zip_rejects_path_traversal_entry() {
        // Hand-build a malicious zip entry whose path contains `..`.
        let archive = make_zip(&[("../etc/passwd", b"x")]);
        let dir = tempdir().unwrap();
        let target = dir.path().join("tldr");
        let result = extract_zip(&archive, &target);
        match result {
            Err(ExtractError::PathEscape(_)) => {}
            other => panic!("expected PathEscape, got {other:?}"),
        }
    }

    #[test]
    fn extract_zip_replaces_existing_target_atomically() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("tldr");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("old.md"), b"stale").unwrap();

        let archive = make_zip(&[("new.md", b"fresh")]);
        extract_zip(&archive, &target).expect("extract");

        assert!(target.join("new.md").is_file());
        assert!(!target.join("old.md").exists(), "old.md must be replaced");
    }
}
