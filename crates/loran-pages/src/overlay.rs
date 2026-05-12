// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Partial-frontmatter overlay support for WP-P2.13.
//!
//! Spec §5.1 says "later layers replace earlier ones field-by-field,
//! not record-by-record (so a user can override `summary` without
//! re-stating `category`, `replaces`, etc.)". An overlay file on disk
//! therefore needs to be parseable with only `name` (the merge key)
//! present — every other frontmatter field is optional.
//!
//! [`OverlayPage::parse`] does that, and [`Page::merge_overlay`]
//! applies the partial onto a base [`Page`]. Empty / unset bodies in
//! the overlay file inherit the base body; non-empty bodies replace it.

use serde::Deserialize;

use crate::error::PageError;
use crate::page::Page;
use crate::parse::{split_frontmatter, validate_category, validate_summary_length};

/// Partial overlay frontmatter. `name` is required (it's the merge
/// key); everything else is `Some(_)` when the overlay sets the field
/// and `None` to inherit from the lower-precedence layer.
///
/// TOML has no `null`, so there is no syntax for "clear an optional
/// field that the base layer set" — an overlay can only override or
/// inherit. That matches Spec §5.1's intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayPage {
    pub name: String,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub replaces: Option<Vec<String>>,
    pub safe_alias_for: Option<Vec<String>>,
    pub pairs_with: Option<Vec<String>>,
    pub official: Option<String>,
    pub tldr_page: Option<String>,
    pub tags: Option<Vec<String>>,
    pub written_in: Option<String>,
    pub language: Option<String>,
    pub since: Option<String>,
    pub aliases: Option<Vec<String>>,
    /// `None` means "inherit body from the base layer"; `Some(_)` (even
    /// `Some("")`) means "replace". The parser maps a whitespace-only
    /// body to `None` so a typical "override metadata, keep upstream
    /// prose" overlay file Just Works.
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOverlay {
    name: Option<String>,
    category: Option<String>,
    summary: Option<String>,
    replaces: Option<Vec<String>>,
    safe_alias_for: Option<Vec<String>>,
    pairs_with: Option<Vec<String>>,
    official: Option<String>,
    tldr_page: Option<String>,
    tags: Option<Vec<String>>,
    written_in: Option<String>,
    language: Option<String>,
    since: Option<String>,
    aliases: Option<Vec<String>>,
}

impl OverlayPage {
    /// Parse an overlay file. Same `+++`-fenced TOML frontmatter format
    /// as [`Page::parse`], but every field except `name` may be absent.
    ///
    /// Field-level invariants are still enforced when the overlay does
    /// set them (`summary` length, `category` shape) so a malformed
    /// overlay file fails fast rather than at merge time.
    pub fn parse(input: &str) -> Result<Self, PageError> {
        let (frontmatter, body) = split_frontmatter(input)?;
        let raw: RawOverlay = toml::from_str(frontmatter)?;

        let name = raw.name.ok_or(PageError::MissingField("name"))?;

        if let Some(summary) = raw.summary.as_deref() {
            validate_summary_length(summary)?;
        }
        if let Some(category) = raw.category.as_deref() {
            validate_category(category)?;
        }

        // safe_alias_for ⊆ replaces only validated when both are set in
        // the same overlay file (so it does not falsely reject an
        // overlay that adds to `safe_alias_for` while inheriting
        // `replaces` from the base layer; the merged Page is
        // re-validated after the merge applies).
        if let (Some(aliases), Some(replaces)) = (&raw.safe_alias_for, &raw.replaces) {
            for alias in aliases {
                if !replaces.iter().any(|r| r == alias) {
                    return Err(PageError::InvalidSafeAliasFor(alias.clone()));
                }
            }
        }

        let body = if body.trim().is_empty() {
            None
        } else {
            Some(body.to_owned())
        };

        Ok(Self {
            name,
            category: raw.category,
            summary: raw.summary,
            replaces: raw.replaces,
            safe_alias_for: raw.safe_alias_for,
            pairs_with: raw.pairs_with,
            official: raw.official,
            tldr_page: raw.tldr_page,
            tags: raw.tags,
            written_in: raw.written_in,
            language: raw.language,
            since: raw.since,
            aliases: raw.aliases,
            body,
        })
    }

    /// Try to promote this overlay into a full [`Page`]. Succeeds only
    /// when every required field is set; otherwise returns the same
    /// [`PageError::MissingField`] that `Page::parse` would.
    pub fn into_full_page(self) -> Result<Page, PageError> {
        let category = self.category.ok_or(PageError::MissingField("category"))?;
        let summary = self.summary.ok_or(PageError::MissingField("summary"))?;

        let replaces = self.replaces.unwrap_or_default();
        let safe_alias_for = self.safe_alias_for.unwrap_or_default();
        for alias in &safe_alias_for {
            if !replaces.iter().any(|r| r == alias) {
                return Err(PageError::InvalidSafeAliasFor(alias.clone()));
            }
        }

        Ok(Page {
            name: self.name,
            category,
            summary,
            replaces,
            safe_alias_for,
            pairs_with: self.pairs_with.unwrap_or_default(),
            official: self.official,
            tldr_page: self.tldr_page,
            tags: self.tags.unwrap_or_default(),
            written_in: self.written_in,
            language: self.language,
            since: self.since,
            aliases: self.aliases.unwrap_or_default(),
            body: self.body.unwrap_or_default(),
        })
    }
}

impl Page {
    /// Apply an [`OverlayPage`]'s set fields onto this page, returning
    /// the merged result. Re-validates every cross-field invariant.
    ///
    /// # Errors
    ///
    /// - [`PageError::InvalidSafeAliasFor`] if the merged `safe_alias_for`
    ///   is no longer a subset of the merged `replaces`.
    /// - [`PageError::SummaryTooLong`] / [`PageError::InvalidCategory`]
    ///   if the overlay's replacement violates the per-field rule
    ///   (these are also checked at overlay-parse time, so the merge-
    ///   time check is a defense-in-depth.)
    ///
    /// # Panics
    ///
    /// Debug builds assert that `overlay.name == self.name` — merging
    /// across names is a programming error in the caller (the layer
    /// engine keys on name).
    pub fn merge_overlay(mut self, overlay: OverlayPage) -> Result<Self, PageError> {
        debug_assert_eq!(
            self.name, overlay.name,
            "Page::merge_overlay called with mismatched names"
        );

        if let Some(category) = overlay.category {
            validate_category(&category)?;
            self.category = category;
        }
        if let Some(summary) = overlay.summary {
            validate_summary_length(&summary)?;
            self.summary = summary;
        }
        if let Some(replaces) = overlay.replaces {
            self.replaces = replaces;
        }
        if let Some(safe_alias_for) = overlay.safe_alias_for {
            self.safe_alias_for = safe_alias_for;
        }
        if let Some(pairs_with) = overlay.pairs_with {
            self.pairs_with = pairs_with;
        }
        if let Some(official) = overlay.official {
            self.official = Some(official);
        }
        if let Some(tldr_page) = overlay.tldr_page {
            self.tldr_page = Some(tldr_page);
        }
        if let Some(tags) = overlay.tags {
            self.tags = tags;
        }
        if let Some(written_in) = overlay.written_in {
            self.written_in = Some(written_in);
        }
        if let Some(language) = overlay.language {
            self.language = Some(language);
        }
        if let Some(since) = overlay.since {
            self.since = Some(since);
        }
        if let Some(aliases) = overlay.aliases {
            self.aliases = aliases;
        }
        if let Some(body) = overlay.body {
            self.body = body;
        }

        for alias in &self.safe_alias_for {
            if !self.replaces.iter().any(|r| r == alias) {
                return Err(PageError::InvalidSafeAliasFor(alias.clone()));
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::OverlayPage;
    use crate::Page;
    use crate::error::PageError;

    fn base_eza() -> Page {
        let src = "+++\n\
            name = \"eza\"\n\
            category = \"file-listing\"\n\
            summary = \"Modern ls replacement.\"\n\
            replaces = [\"ls\"]\n\
            tags = [\"filesystem\"]\n\
            official = \"https://eza.rocks\"\n\
            +++\n\
            \n\
            Upstream body.\n";
        Page::parse(src).expect("base page parses")
    }

    #[test]
    fn name_only_overlay_parses_and_inherits_everything() {
        let src = "+++\nname = \"eza\"\n+++\n";
        let overlay = OverlayPage::parse(src).expect("name-only overlay must parse");
        assert_eq!(overlay.name, "eza");
        assert!(overlay.category.is_none());
        assert!(overlay.summary.is_none());
        assert!(overlay.replaces.is_none());
        assert!(overlay.body.is_none());

        let merged = base_eza().merge_overlay(overlay).expect("merge ok");
        assert_eq!(merged.category, "file-listing");
        assert_eq!(merged.summary, "Modern ls replacement.");
        assert_eq!(merged.replaces, vec!["ls".to_owned()]);
        assert_eq!(merged.body.trim(), "Upstream body.");
    }

    #[test]
    fn overlay_overrides_summary_without_restating_category() {
        let src = "+++\nname = \"eza\"\nsummary = \"Steelbore-tuned ls.\"\n+++\n";
        let overlay = OverlayPage::parse(src).unwrap();
        let merged = base_eza().merge_overlay(overlay).unwrap();
        assert_eq!(merged.summary, "Steelbore-tuned ls.");
        assert_eq!(merged.category, "file-listing");
        assert_eq!(merged.replaces, vec!["ls"]);
    }

    #[test]
    fn overlay_replaces_vec_field_wholly() {
        let src = "+++\nname = \"eza\"\nreplaces = [\"ls\", \"dir\"]\n+++\n";
        let overlay = OverlayPage::parse(src).unwrap();
        let merged = base_eza().merge_overlay(overlay).unwrap();
        assert_eq!(merged.replaces, vec!["ls", "dir"]);
        // tags untouched.
        assert_eq!(merged.tags, vec!["filesystem"]);
    }

    #[test]
    fn overlay_body_with_content_replaces_base_body() {
        let src = "+++\nname = \"eza\"\n+++\n\nDistro-specific notes.\n";
        let overlay = OverlayPage::parse(src).unwrap();
        let merged = base_eza().merge_overlay(overlay).unwrap();
        assert!(merged.body.contains("Distro-specific notes"));
        assert!(!merged.body.contains("Upstream body"));
    }

    #[test]
    fn overlay_whitespace_only_body_inherits_base_body() {
        let src = "+++\nname = \"eza\"\n+++\n\n   \n\t\n";
        let overlay = OverlayPage::parse(src).unwrap();
        assert!(overlay.body.is_none(), "whitespace body parses as None");
        let merged = base_eza().merge_overlay(overlay).unwrap();
        assert!(merged.body.contains("Upstream body"));
    }

    #[test]
    fn overlay_missing_name_fails() {
        let src = "+++\ncategory = \"c\"\n+++\n";
        let err = OverlayPage::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::MissingField("name")),
            "got {err:?}"
        );
    }

    #[test]
    fn overlay_rejects_unknown_fields() {
        let src = "+++\nname = \"eza\"\nrepleaces = []\n+++\n";
        let err = OverlayPage::parse(src).unwrap_err();
        assert!(matches!(err, PageError::InvalidToml(_)), "got {err:?}");
    }

    #[test]
    fn overlay_validates_summary_length_at_parse_time() {
        let long = "x".repeat(121);
        let src = format!("+++\nname = \"x\"\nsummary = \"{long}\"\n+++\n");
        let err = OverlayPage::parse(&src).unwrap_err();
        assert!(
            matches!(err, PageError::SummaryTooLong { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn overlay_validates_category_shape_at_parse_time() {
        let src = "+++\nname = \"x\"\ncategory = \"/bad\"\n+++\n";
        let err = OverlayPage::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::InvalidCategory { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn safe_alias_subset_violation_caught_post_merge() {
        // Base sets replaces=[ls]; overlay sets safe_alias_for=[dir]
        // (not in base.replaces, and overlay doesn't touch replaces).
        let src = "+++\nname = \"eza\"\nsafe_alias_for = [\"dir\"]\n+++\n";
        let overlay = OverlayPage::parse(src).unwrap();
        let err = base_eza().merge_overlay(overlay).unwrap_err();
        match err {
            PageError::InvalidSafeAliasFor(name) => assert_eq!(name, "dir"),
            other => panic!("expected InvalidSafeAliasFor, got {other:?}"),
        }
    }

    #[test]
    fn overlay_with_safe_alias_subset_of_local_replaces_is_ok() {
        let src = "+++\n\
            name = \"eza\"\n\
            replaces = [\"ls\", \"dir\"]\n\
            safe_alias_for = [\"dir\"]\n\
            +++\n";
        let overlay = OverlayPage::parse(src).unwrap();
        let merged = base_eza().merge_overlay(overlay).unwrap();
        assert_eq!(merged.replaces, vec!["ls", "dir"]);
        assert_eq!(merged.safe_alias_for, vec!["dir"]);
    }

    #[test]
    fn into_full_page_succeeds_when_required_fields_present() {
        let src = "+++\n\
            name = \"btop\"\n\
            category = \"system-monitoring\"\n\
            summary = \"Resource monitor.\"\n\
            +++\n";
        let overlay = OverlayPage::parse(src).unwrap();
        let page = overlay.into_full_page().expect("complete overlay → Page");
        assert_eq!(page.name, "btop");
        assert_eq!(page.category, "system-monitoring");
    }

    #[test]
    fn into_full_page_fails_when_required_fields_absent() {
        let src = "+++\nname = \"eza\"\n+++\n";
        let overlay = OverlayPage::parse(src).unwrap();
        let err = overlay.into_full_page().unwrap_err();
        assert!(matches!(err, PageError::MissingField(_)), "got {err:?}");
    }
}
