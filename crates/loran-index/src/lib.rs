// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

#![deny(unsafe_code)]

//! Loran index — pluggable ingestion of [`loran_pages::Page`] values and
//! the merged in-memory [`Index`] queried by the CLI surface.
//!
//! ## Architecture
//!
//! Ingestion is decoupled from indexing via the [`Ingestor`] trait, so
//! the same [`Index::build`] pipeline can consume pages produced from
//! any source. v1 ships exactly one implementation
//! ([`MarkdownPagesIngestor`], which walks a directory of `*.md`
//! files); Phase 3 will add a `DescribeIngestor` that synthesises
//! entries by invoking `<tool> describe --json` against CLI-Standard-compliant
//! Spacecraft Software CLIs on `$PATH`.
//!
//! ## Failure policy
//!
//! Both stages **fail loud** (Spec §6.1, FR-035):
//!
//! - [`MarkdownPagesIngestor::ingest`] returns the first malformed page
//!   it encounters wrapped in [`IngestError::Page`]. There is no silent
//!   skip mode.
//! - [`Index::build`] rejects duplicate `name`s with
//!   [`IndexError::DuplicateName`]. The caller is expected to surface
//!   this as a `PAGE_PARSE_ERROR` (Spec §9 code 8).

mod describe;
mod distro;
mod error;
mod index;
mod ingestor;
mod markdown;
mod overlay;

pub use describe::{
    DEFAULT_TIMEOUT as DESCRIBE_DEFAULT_TIMEOUT, DescribeError, DescribeIngestor, RealRunner,
    Runner, SYNTH_CATEGORY,
};
pub use distro::{
    DEFAULT_OS_RELEASE_PATH, DISTRO_GENERIC, detect_distro_id, detect_distro_id_from,
};
pub use error::{IndexError, IngestError};
pub use index::Index;
pub use ingestor::Ingestor;
pub use markdown::MarkdownPagesIngestor;
pub use overlay::{LayeredIngestor, OverlayLayer};
