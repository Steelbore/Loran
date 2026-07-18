// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! The pluggable [`Ingestor`] trait.

use loran_pages::Page;

use crate::error::IngestError;

/// A source of [`Page`] values for index construction.
///
/// `Ingestor` is the single seam between the loader pipeline and the
/// underlying source format. v1 implementations:
///
/// - [`crate::MarkdownPagesIngestor`] — walks a directory of `*.md`
///   files. The only implementation shipped in Phase 1.
///
/// Future implementations (Phase 3):
///
/// - `DescribeIngestor` — invokes `<tool> describe --json` against
///   CLI-Standard-compliant Spacecraft Software binaries on `$PATH` and synthesises
///   baseline entries. Lets the Spacecraft Software ecosystem self-document by
///   default; curated pages overlay on top where they exist.
pub trait Ingestor {
    /// Produce every page this source can produce, in unspecified order.
    ///
    /// On the first malformed page or I/O failure the call returns
    /// `Err`; there is no silent-skip mode (Spec §6.1, FR-035).
    fn ingest(&self) -> Result<Vec<Page>, IngestError>;
}

#[cfg(test)]
mod tests {
    use loran_pages::Page;

    use super::{IngestError, Ingestor};

    /// Minimal mock used to verify the trait surface compiles and is
    /// object-safe via `&dyn Ingestor`.
    struct CannedIngestor(Result<Vec<Page>, IngestError>);

    impl Ingestor for CannedIngestor {
        fn ingest(&self) -> Result<Vec<Page>, IngestError> {
            match &self.0 {
                Ok(pages) => Ok(pages.clone()),
                Err(IngestError::BadSource(s)) => Err(IngestError::BadSource(s.clone())),
                _ => unreachable!("test mock supplies only Ok or BadSource"),
            }
        }
    }

    fn make_page(name: &str) -> Page {
        let src = format!("+++\nname = \"{name}\"\ncategory = \"c\"\nsummary = \"s\"\n+++\n");
        Page::parse(&src).expect("test page builds")
    }

    #[test]
    fn trait_is_object_safe() {
        let pages = vec![make_page("a"), make_page("b")];
        let ingestor: Box<dyn Ingestor> = Box::new(CannedIngestor(Ok(pages)));
        let collected = ingestor.ingest().expect("mock returns Ok");
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn mock_can_surface_bad_source_error() {
        let ingestor = CannedIngestor(Err(IngestError::BadSource("nope".into())));
        match ingestor.ingest() {
            Err(IngestError::BadSource(s)) => assert_eq!(s, "nope"),
            other => panic!("expected BadSource, got {other:?}"),
        }
    }
}
