// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! [`Index`] — merged, lookup-friendly view of every page in the catalog.

use std::collections::BTreeMap;

use loran_pages::Page;
use serde::{Deserialize, Serialize};

use crate::error::IndexError;

/// In-memory catalog index.
///
/// Constructed via [`Index::build`] from a `Vec<Page>` produced by an
/// [`crate::Ingestor`]. The primary store is a `BTreeMap<String, Page>`
/// keyed by canonical [`Page::name`]; secondary indexes are populated
/// in the same build pass so the CLI surface can answer
/// "list by category", "find by legacy name", and "search by tag" in
/// O(log N) without re-traversing the page set.
///
/// Secondary indexes store **tool names** (the primary key) rather than
/// references to `Page` values, keeping the index trivially `Clone`-able
/// and avoiding self-referential lifetime gymnastics. Callers retrieve
/// full `Page` data via [`Index::get`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)] // the `by_` prefix names the lookup axis
pub struct Index {
    by_name: BTreeMap<String, Page>,
    by_category: BTreeMap<String, Vec<String>>,
    by_replaces: BTreeMap<String, Vec<String>>,
    by_safe_alias: BTreeMap<String, Vec<String>>,
    by_tag: BTreeMap<String, Vec<String>>,
}

impl Index {
    /// Construct an index from a flat list of pages.
    ///
    /// Returns [`IndexError::DuplicateName`] if two pages share a
    /// [`Page::name`]. Pages are otherwise consumed in input order,
    /// and the secondary indexes preserve that order within each bucket
    /// — useful for stable test output.
    pub fn build(pages: Vec<Page>) -> Result<Self, IndexError> {
        let mut idx = Self::default();

        for page in pages {
            if idx.by_name.contains_key(&page.name) {
                return Err(IndexError::DuplicateName(page.name));
            }

            idx.by_category
                .entry(page.category.clone())
                .or_default()
                .push(page.name.clone());

            for legacy in &page.replaces {
                idx.by_replaces
                    .entry(legacy.clone())
                    .or_default()
                    .push(page.name.clone());
            }

            for legacy in &page.safe_alias_for {
                idx.by_safe_alias
                    .entry(legacy.clone())
                    .or_default()
                    .push(page.name.clone());
            }

            for tag in &page.tags {
                idx.by_tag
                    .entry(tag.clone())
                    .or_default()
                    .push(page.name.clone());
            }

            idx.by_name.insert(page.name.clone(), page);
        }

        Ok(idx)
    }

    /// Look up a page by its canonical name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Page> {
        self.by_name.get(name)
    }

    /// Iterate every page in the index, sorted by name.
    pub fn all(&self) -> impl Iterator<Item = &Page> {
        self.by_name.values()
    }

    /// Number of pages held by the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// `true` if the index has no pages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// All pages in a given category.
    ///
    /// Categories are slash-tolerant strings; `category` is matched
    /// exactly (no prefix/suffix logic) because the v1 UX renders flat
    /// (Spec §2.16). Returns an empty iterator for unknown categories.
    pub fn by_category(&self, category: &str) -> impl Iterator<Item = &Page> {
        self.by_replaces_or_category(&self.by_category, category)
    }

    /// All pages that supersede a given legacy tool name.
    ///
    /// `loran find ls` is the primary consumer. Returns an empty
    /// iterator if no entry declares `legacy` in its `replaces` list.
    pub fn by_replaces(&self, legacy: &str) -> impl Iterator<Item = &Page> {
        self.by_replaces_or_category(&self.by_replaces, legacy)
    }

    /// All pages that declare a given legacy tool name in `safe_alias_for`.
    ///
    /// Strict subset of [`Self::by_replaces`] — by the Spec §6.1
    /// invariant `safe_alias_for ⊆ replaces`, but with the additional
    /// promise that the modern entry's default behaviour is close
    /// enough to the legacy one to back an `alias legacy=modern`. Used
    /// by `loran find --safe-alias` and the future `loran list
    /// --safe-alias-for`.
    pub fn by_safe_alias(&self, legacy: &str) -> impl Iterator<Item = &Page> {
        self.by_replaces_or_category(&self.by_safe_alias, legacy)
    }

    /// All pages carrying a given discovery tag.
    pub fn by_tag(&self, tag: &str) -> impl Iterator<Item = &Page> {
        self.by_replaces_or_category(&self.by_tag, tag)
    }

    /// Every category present in the index, sorted.
    pub fn categories(&self) -> impl Iterator<Item = &str> {
        self.by_category.keys().map(String::as_str)
    }

    /// Number of pages in a given category. `0` for unknown categories.
    pub fn category_count(&self, category: &str) -> usize {
        self.by_category.get(category).map_or(0, Vec::len)
    }

    fn by_replaces_or_category<'a>(
        &'a self,
        secondary: &'a BTreeMap<String, Vec<String>>,
        key: &str,
    ) -> impl Iterator<Item = &'a Page> {
        secondary
            .get(key)
            .into_iter()
            .flat_map(|names| names.iter())
            .filter_map(|name| self.by_name.get(name))
    }
}

#[cfg(test)]
mod tests {
    use loran_pages::Page;

    use super::{Index, IndexError};

    fn make_page_full(name: &str, category: &str, replaces: &[&str], tags: &[&str]) -> Page {
        let replaces_lit = replaces
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let tags_lit = tags
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let src = format!(
            "+++\n\
             name = \"{name}\"\n\
             category = \"{category}\"\n\
             summary = \"s\"\n\
             replaces = [{replaces_lit}]\n\
             tags = [{tags_lit}]\n\
             +++\n"
        );
        Page::parse(&src).expect("test page builds")
    }

    #[test]
    fn build_indexes_three_pages_and_supports_get() {
        let pages = vec![
            make_page_full("eza", "file-listing", &["ls"], &["filesystem"]),
            make_page_full("bat", "file-viewing", &["cat"], &["filesystem"]),
            make_page_full("rg", "text-search", &["grep"], &["search"]),
        ];

        let idx = Index::build(pages).expect("build succeeds");

        assert_eq!(idx.len(), 3);
        assert!(!idx.is_empty());
        assert_eq!(idx.get("eza").unwrap().category, "file-listing");
        assert_eq!(idx.get("bat").unwrap().replaces, vec!["cat"]);
        assert!(idx.get("unknown").is_none());
    }

    #[test]
    fn build_rejects_duplicate_names() {
        let pages = vec![
            make_page_full("eza", "file-listing", &["ls"], &[]),
            make_page_full("eza", "file-viewing", &["dir"], &[]),
        ];
        let err = Index::build(pages).unwrap_err();
        match err {
            IndexError::DuplicateName(name) => assert_eq!(name, "eza"),
            IndexError::Ingest(other) => panic!("expected DuplicateName, got Ingest({other:?})"),
        }
    }

    #[test]
    fn by_category_returns_matching_pages_only() {
        let idx = Index::build(vec![
            make_page_full("eza", "file-listing", &[], &[]),
            make_page_full("lsd", "file-listing", &[], &[]),
            make_page_full("bat", "file-viewing", &[], &[]),
        ])
        .unwrap();

        let mut names: Vec<&str> = idx
            .by_category("file-listing")
            .map(|p| p.name.as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(names, vec!["eza", "lsd"]);

        let empty: Vec<&str> = idx
            .by_category("no-such-category")
            .map(|p| p.name.as_str())
            .collect();
        assert!(empty.is_empty());
    }

    #[test]
    fn by_replaces_supports_multimap_results() {
        let idx = Index::build(vec![
            make_page_full("rg", "text-search", &["grep"], &[]),
            make_page_full("ack", "text-search", &["grep"], &[]),
            make_page_full("bat", "file-viewing", &["cat"], &[]),
        ])
        .unwrap();

        let mut grep_alternatives: Vec<&str> =
            idx.by_replaces("grep").map(|p| p.name.as_str()).collect();
        grep_alternatives.sort_unstable();
        assert_eq!(grep_alternatives, vec!["ack", "rg"]);

        let cat_alternatives: Vec<&str> = idx.by_replaces("cat").map(|p| p.name.as_str()).collect();
        assert_eq!(cat_alternatives, vec!["bat"]);

        let absent: Vec<&str> = idx.by_replaces("vi").map(|p| p.name.as_str()).collect();
        assert!(absent.is_empty());
    }

    #[test]
    fn by_safe_alias_is_a_strict_subset_of_by_replaces() {
        let make_page = |name: &str, replaces: &[&str], safe: &[&str]| {
            let replaces_lit = replaces
                .iter()
                .map(|r| format!("\"{r}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let safe_lit = safe
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let src = format!(
                "+++\nname = \"{name}\"\ncategory = \"c\"\nsummary = \"s\"\n\
                 replaces = [{replaces_lit}]\nsafe_alias_for = [{safe_lit}]\n+++\n"
            );
            Page::parse(&src).expect("test page builds")
        };

        let idx = Index::build(vec![
            // bat: alias-safe for cat (drop-in)
            make_page("bat", &["cat"], &["cat"]),
            // eza: replaces ls but NOT alias-safe (different columns)
            make_page("eza", &["ls"], &[]),
            // jaq: replaces jq but NOT alias-safe (no auto-vivification,
            // 22 missing builtins, 9 rejected flags)
            make_page("jaq", &["jq"], &[]),
        ])
        .unwrap();

        // by_replaces sees every entry that supersedes the legacy name.
        let cat_replacements: Vec<&str> = idx.by_replaces("cat").map(|p| p.name.as_str()).collect();
        assert_eq!(cat_replacements, vec!["bat"]);

        // by_safe_alias sees only the strict subset that's actually alias-safe.
        let cat_alias_safe: Vec<&str> = idx.by_safe_alias("cat").map(|p| p.name.as_str()).collect();
        assert_eq!(cat_alias_safe, vec!["bat"]);

        // ls has a replacer but no alias-safe replacer.
        let ls_replacements: Vec<&str> = idx.by_replaces("ls").map(|p| p.name.as_str()).collect();
        assert_eq!(ls_replacements, vec!["eza"]);
        let ls_alias_safe: Vec<&str> = idx.by_safe_alias("ls").map(|p| p.name.as_str()).collect();
        assert!(ls_alias_safe.is_empty());
    }

    #[test]
    fn by_tag_returns_pages_sharing_a_tag() {
        let idx = Index::build(vec![
            make_page_full("eza", "file-listing", &[], &["filesystem"]),
            make_page_full("bat", "file-viewing", &[], &["filesystem", "syntax"]),
            make_page_full("rg", "text-search", &[], &["search"]),
        ])
        .unwrap();

        let mut filesystem: Vec<&str> = idx.by_tag("filesystem").map(|p| p.name.as_str()).collect();
        filesystem.sort_unstable();
        assert_eq!(filesystem, vec!["bat", "eza"]);
    }

    #[test]
    fn categories_iterator_lists_distinct_categories_sorted() {
        let idx = Index::build(vec![
            make_page_full("eza", "file-listing", &[], &[]),
            make_page_full("lsd", "file-listing", &[], &[]),
            make_page_full("rg", "text-search", &[], &[]),
        ])
        .unwrap();

        let cats: Vec<&str> = idx.categories().collect();
        assert_eq!(cats, vec!["file-listing", "text-search"]);
        assert_eq!(idx.category_count("file-listing"), 2);
        assert_eq!(idx.category_count("text-search"), 1);
        assert_eq!(idx.category_count("missing"), 0);
    }

    #[test]
    fn all_iterates_in_name_sorted_order() {
        let idx = Index::build(vec![
            make_page_full("rg", "text-search", &[], &[]),
            make_page_full("bat", "file-viewing", &[], &[]),
            make_page_full("eza", "file-listing", &[], &[]),
        ])
        .unwrap();

        let names: Vec<&str> = idx.all().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["bat", "eza", "rg"]);
    }

    #[test]
    fn empty_index_is_well_defined() {
        let idx = Index::build(Vec::new()).unwrap();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
        assert_eq!(idx.all().count(), 0);
        assert_eq!(idx.categories().count(), 0);
    }
}
