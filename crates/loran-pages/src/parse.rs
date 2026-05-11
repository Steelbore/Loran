// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Public parse entry point and the validation pipeline that backs it.

use crate::error::PageError;
use crate::page::{Page, RawPage};

/// The literal frontmatter fence (Hugo / Zola style).
const FENCE: &str = "+++";

/// Maximum length of `summary`, in Unicode scalar values (Spec §6.1).
const SUMMARY_MAX_CHARS: usize = 120;

impl Page {
    /// Parse a Loran page from its on-disk form.
    ///
    /// `input` is the full file contents: a `+++`-fenced TOML frontmatter
    /// block followed by a Markdown body. Returns a validated [`Page`]
    /// or a typed [`PageError`] describing the first failure encountered.
    ///
    /// Validation order, in order of fast-fail:
    ///
    /// 1. Frontmatter fence pair located (or [`PageError::MissingFrontmatter`]).
    /// 2. TOML deserialise (or [`PageError::InvalidToml`]).
    /// 3. Required fields present (or [`PageError::MissingField`]).
    /// 4. `summary` length cap (or [`PageError::SummaryTooLong`]).
    /// 5. `category` well-formedness (or [`PageError::InvalidCategory`]).
    /// 6. `safe_alias_for ⊆ replaces` (or [`PageError::InvalidSafeAliasFor`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use loran_pages::Page;
    ///
    /// let src = "\
    /// +++
    /// name = \"eza\"
    /// category = \"file-listing\"
    /// summary = \"Modern ls replacement.\"
    /// +++
    ///
    /// Body content here.
    /// ";
    /// let page = Page::parse(src).expect("valid fixture");
    /// assert_eq!(page.name, "eza");
    /// assert_eq!(page.category, "file-listing");
    /// ```
    pub fn parse(input: &str) -> Result<Self, PageError> {
        let (frontmatter, body) = split_frontmatter(input)?;
        let raw: RawPage = toml::from_str(frontmatter)?;
        validate(raw, body)
    }
}

/// Locate the opening and closing `+++` fences, returning
/// `(frontmatter_toml, body_after_closing_fence)`.
///
/// The opening fence must be the first non-empty line of the file.
/// A leading UTF-8 BOM is tolerated by trimming `input` before the fence
/// search.
fn split_frontmatter(input: &str) -> Result<(&str, &str), PageError> {
    let trimmed = input.strip_prefix('\u{feff}').unwrap_or(input);

    // Locate the opening fence at the very start (after optional whitespace
    // and a possible single blank line).
    let Some(after_open) = trimmed.strip_prefix(FENCE) else {
        return Err(PageError::MissingFrontmatter);
    };

    // The character right after `+++` must be a newline; we don't accept
    // `+++foo` as a valid opener.
    let Some(after_open) = after_open.strip_prefix('\n') else {
        return Err(PageError::MissingFrontmatter);
    };

    // Find the closing fence. The closing `+++` must sit on its own line
    // — `\n+++\n` or `\n+++` at EOF. Search for `\n+++` and require the
    // following byte to be either `\n` or EOF.
    let needle = "\n+++";
    let mut search_from = 0usize;
    let close_idx = loop {
        match after_open[search_from..].find(needle) {
            Some(rel) => {
                let absolute = search_from + rel;
                let after = absolute + needle.len();
                let is_eof = after == after_open.len();
                let next_is_newline = after_open.as_bytes().get(after) == Some(&b'\n');
                if is_eof || next_is_newline {
                    break absolute;
                }
                // It's `+++` followed by something else (e.g. `+++foo`);
                // keep searching past this position.
                search_from = absolute + needle.len();
            }
            None => return Err(PageError::MissingFrontmatter),
        }
    };

    let frontmatter = &after_open[..close_idx];
    // Skip past `\n+++` and the optional trailing `\n`.
    let body_start = close_idx + needle.len();
    let body_start = if after_open.as_bytes().get(body_start) == Some(&b'\n') {
        body_start + 1
    } else {
        body_start
    };
    let body = &after_open[body_start..];

    Ok((frontmatter, body))
}

/// Validate a deserialised [`RawPage`] into a [`Page`].
fn validate(raw: RawPage, body: &str) -> Result<Page, PageError> {
    let name = raw.name.ok_or(PageError::MissingField("name"))?;
    let category = raw.category.ok_or(PageError::MissingField("category"))?;
    let summary = raw.summary.ok_or(PageError::MissingField("summary"))?;

    let summary_chars = summary.chars().count();
    if summary_chars > SUMMARY_MAX_CHARS {
        return Err(PageError::SummaryTooLong {
            actual_chars: summary_chars,
        });
    }

    validate_category(&category)?;

    for alias in &raw.safe_alias_for {
        if !raw.replaces.iter().any(|r| r == alias) {
            return Err(PageError::InvalidSafeAliasFor(alias.clone()));
        }
    }

    Ok(Page {
        name,
        category,
        summary,
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
        body: body.to_owned(),
    })
}

/// Enforce category well-formedness per Spec §6.1.
fn validate_category(value: &str) -> Result<(), PageError> {
    let make_err = |reason: &'static str| PageError::InvalidCategory {
        value: value.to_owned(),
        reason,
    };

    if value.is_empty() {
        return Err(make_err("must be a non-empty string"));
    }
    if value.starts_with('/') {
        return Err(make_err("must not start with `/`"));
    }
    if value.ends_with('/') {
        return Err(make_err("must not end with `/`"));
    }
    if value.contains("//") {
        return Err(make_err("must not contain `//`"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Page;
    use crate::error::PageError;

    // ───── Valid-page fixtures ─────────────────────────────────────

    #[test]
    fn parses_minimum_viable_page() {
        let src = "\
+++
name = \"eza\"
category = \"file-listing\"
summary = \"Modern ls replacement.\"
+++
";
        let page = Page::parse(src).expect("minimum viable page must parse");
        assert_eq!(page.name, "eza");
        assert_eq!(page.category, "file-listing");
        assert_eq!(page.summary, "Modern ls replacement.");
        assert!(page.replaces.is_empty());
        assert!(page.safe_alias_for.is_empty());
        assert!(page.pairs_with.is_empty());
        assert!(page.tags.is_empty());
        assert!(page.aliases.is_empty());
        assert!(page.official.is_none());
        assert!(page.tldr_page.is_none());
        assert!(page.written_in.is_none());
        assert!(page.language.is_none());
        assert!(page.since.is_none());
        assert!(page.body.is_empty());
    }

    #[test]
    fn parses_full_schema_page() {
        let src = "\
+++
name = \"eza\"
category = \"file-listing\"
replaces = [\"ls\"]
safe_alias_for = []
pairs_with = [\"bat\", \"fd\"]
summary = \"Modern ls replacement. Steelbore default for file listing.\"
official = \"https://eza.rocks\"
tldr_page = \"eza\"
written_in = \"rust\"
since = \"bravais@0.1\"
tags = [\"filesystem\", \"tui-friendly\"]
aliases = [\"exa\"]
+++

## Steelbore Notes

eza is the Steelbore-canonical file lister.
";
        let page = Page::parse(src).expect("full-schema page must parse");
        assert_eq!(page.name, "eza");
        assert_eq!(page.replaces, vec!["ls"]);
        assert!(page.safe_alias_for.is_empty());
        assert_eq!(page.pairs_with, vec!["bat", "fd"]);
        assert_eq!(page.official.as_deref(), Some("https://eza.rocks"));
        assert_eq!(page.tldr_page.as_deref(), Some("eza"));
        assert_eq!(page.written_in.as_deref(), Some("rust"));
        assert_eq!(page.since.as_deref(), Some("bravais@0.1"));
        assert_eq!(page.tags, vec!["filesystem", "tui-friendly"]);
        assert_eq!(page.aliases, vec!["exa"]);
        assert!(page.body.starts_with("\n## Steelbore Notes"));
    }

    #[test]
    fn parses_slash_tolerant_category() {
        let src = "\
+++
name = \"bottom\"
category = \"system/system-monitoring\"
summary = \"Cross-platform system monitor.\"
+++
";
        let page = Page::parse(src).expect("slash-tolerant category must parse");
        assert_eq!(page.category, "system/system-monitoring");
    }

    #[test]
    fn parses_multi_replaces() {
        let src = "\
+++
name = \"procs\"
category = \"process-management\"
summary = \"Modern ps replacement.\"
replaces = [\"ps\", \"pgrep\", \"pidof\"]
+++
";
        let page = Page::parse(src).expect("multi-replaces page must parse");
        assert_eq!(page.replaces, vec!["ps", "pgrep", "pidof"]);
    }

    #[test]
    fn parses_with_pairs_with() {
        let src = "\
+++
name = \"bat\"
category = \"file-viewing\"
summary = \"cat with wings.\"
pairs_with = [\"eza\", \"rg\"]
+++
";
        let page = Page::parse(src).expect("page with pairs_with must parse");
        assert_eq!(page.pairs_with, vec!["eza", "rg"]);
    }

    #[test]
    fn parses_with_safe_alias_for_subset_of_replaces() {
        let src = "\
+++
name = \"bat\"
category = \"file-viewing\"
summary = \"cat with wings.\"
replaces = [\"cat\", \"less\"]
safe_alias_for = [\"cat\"]
+++
";
        let page = Page::parse(src).expect("alias-safe subset must parse");
        assert_eq!(page.safe_alias_for, vec!["cat"]);
        assert_eq!(page.replaces, vec!["cat", "less"]);
    }

    #[test]
    fn tolerates_utf8_bom() {
        let src = "\u{feff}+++\nname = \"x\"\ncategory = \"c\"\nsummary = \"s\"\n+++\n";
        let page = Page::parse(src).expect("BOM-prefixed input must parse");
        assert_eq!(page.name, "x");
    }

    #[test]
    fn body_captured_verbatim_after_closing_fence() {
        let src = "\
+++
name = \"x\"
category = \"c\"
summary = \"s\"
+++
body line 1
body line 2
";
        let page = Page::parse(src).expect("body capture must parse");
        assert_eq!(page.body, "body line 1\nbody line 2\n");
    }

    // ───── Error-path fixtures (one per PageError variant) ─────────

    #[test]
    fn error_missing_frontmatter_no_fences() {
        let src = "just a markdown body with no frontmatter";
        let err = Page::parse(src).unwrap_err();
        assert!(matches!(err, PageError::MissingFrontmatter), "got {err:?}");
    }

    #[test]
    fn error_missing_frontmatter_unclosed_fence() {
        let src = "+++\nname = \"x\"\n";
        let err = Page::parse(src).unwrap_err();
        assert!(matches!(err, PageError::MissingFrontmatter), "got {err:?}");
    }

    #[test]
    fn error_invalid_toml() {
        let src = "+++\nname = this is not valid toml\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(matches!(err, PageError::InvalidToml(_)), "got {err:?}");
    }

    #[test]
    fn error_missing_field_name() {
        let src = "+++\ncategory = \"c\"\nsummary = \"s\"\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::MissingField("name")),
            "got {err:?}"
        );
    }

    #[test]
    fn error_missing_field_category() {
        let src = "+++\nname = \"x\"\nsummary = \"s\"\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::MissingField("category")),
            "got {err:?}"
        );
    }

    #[test]
    fn error_missing_field_summary() {
        let src = "+++\nname = \"x\"\ncategory = \"c\"\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::MissingField("summary")),
            "got {err:?}"
        );
    }

    #[test]
    fn error_summary_too_long() {
        let long = "x".repeat(121);
        let src = format!("+++\nname = \"x\"\ncategory = \"c\"\nsummary = \"{long}\"\n+++\n");
        let err = Page::parse(&src).unwrap_err();
        match err {
            PageError::SummaryTooLong { actual_chars } => assert_eq!(actual_chars, 121),
            other => panic!("expected SummaryTooLong, got {other:?}"),
        }
    }

    #[test]
    fn summary_exactly_at_limit_is_accepted() {
        let exactly = "x".repeat(120);
        let src = format!("+++\nname = \"x\"\ncategory = \"c\"\nsummary = \"{exactly}\"\n+++\n");
        let page = Page::parse(&src).expect("summary at the cap must parse");
        assert_eq!(page.summary.chars().count(), 120);
    }

    #[test]
    fn error_safe_alias_for_not_subset_of_replaces() {
        let src = "\
+++
name = \"bat\"
category = \"c\"
summary = \"s\"
replaces = [\"cat\"]
safe_alias_for = [\"less\"]
+++
";
        let err = Page::parse(src).unwrap_err();
        match err {
            PageError::InvalidSafeAliasFor(name) => assert_eq!(name, "less"),
            other => panic!("expected InvalidSafeAliasFor, got {other:?}"),
        }
    }

    #[test]
    fn error_invalid_category_empty() {
        let src = "+++\nname = \"x\"\ncategory = \"\"\nsummary = \"s\"\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::InvalidCategory { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn error_invalid_category_leading_slash() {
        let src = "+++\nname = \"x\"\ncategory = \"/system\"\nsummary = \"s\"\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::InvalidCategory { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn error_invalid_category_trailing_slash() {
        let src = "+++\nname = \"x\"\ncategory = \"system/\"\nsummary = \"s\"\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::InvalidCategory { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn error_invalid_category_double_slash() {
        let src = "+++\nname = \"x\"\ncategory = \"a//b\"\nsummary = \"s\"\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(
            matches!(err, PageError::InvalidCategory { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn unknown_frontmatter_field_is_rejected_as_invalid_toml() {
        // Unknown fields trigger serde(deny_unknown_fields), which serde
        // reports through the toml::Deserializer — we surface that as
        // InvalidToml because it originates from the deserialiser, not
        // from our schema validation layer.
        let src = "+++\nname = \"x\"\ncategory = \"c\"\nsummary = \"s\"\nrepleaces = []\n+++\n";
        let err = Page::parse(src).unwrap_err();
        assert!(matches!(err, PageError::InvalidToml(_)), "got {err:?}");
    }
}
