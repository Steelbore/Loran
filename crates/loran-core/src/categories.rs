// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! Parsed view of the bundled `pages/categories.toml` registry.
//!
//! The build script bakes `categories.toml` into the binary as
//! `BUNDLED_CATEGORIES: &str`; this module parses it into a
//! [`Categories`] map and exposes a [`bundled_categories`] entry
//! point so the `loran categories` sub-command can join the static
//! registry to live counts from [`crate::Index`].

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bundled_data::BUNDLED_CATEGORIES;

/// A single entry in the categories registry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CategoryEntry {
    /// Human-readable title (e.g. `"File listing"`).
    pub title: String,
    /// One-sentence description shown alongside the title.
    pub description: String,
}

/// The full registry — `slug -> CategoryEntry`.
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct Categories {
    /// Entries keyed by slug, sorted (`BTreeMap` preserves order).
    pub entries: BTreeMap<String, CategoryEntry>,
}

impl Categories {
    /// Number of categories declared in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up a single category by slug.
    #[must_use]
    pub fn get(&self, slug: &str) -> Option<&CategoryEntry> {
        self.entries.get(slug)
    }

    /// Iterate `(slug, entry)` pairs in slug order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &CategoryEntry)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Parse the build-time-bundled `BUNDLED_CATEGORIES` string.
///
/// # Errors
///
/// Returns the underlying [`toml::de::Error`] if `BUNDLED_CATEGORIES`
/// fails to deserialise. Build-time validation does not catch this —
/// only individual page files are validated by the build script — so
/// a malformed `categories.toml` would surface at first runtime call.
pub fn bundled_categories() -> Result<Categories, toml::de::Error> {
    let raw: BTreeMap<String, CategoryEntry> = toml::from_str(BUNDLED_CATEGORIES)?;
    Ok(Categories { entries: raw })
}

#[cfg(test)]
mod tests {
    use super::bundled_categories;

    #[test]
    fn bundled_categories_parse_cleanly() {
        let cats = bundled_categories().expect("bundled categories.toml must parse");
        assert!(
            !cats.is_empty(),
            "bundled registry must declare ≥1 category"
        );
    }

    #[test]
    fn seed_registry_includes_file_listing_with_expected_metadata() {
        let cats = bundled_categories().unwrap();
        let entry = cats
            .get("file-listing")
            .expect("file-listing must remain registered");
        assert_eq!(entry.title, "File listing");
        assert!(!entry.description.is_empty());
    }

    #[test]
    fn iter_emits_slug_sorted_pairs() {
        let cats = bundled_categories().unwrap();
        let slugs: Vec<&str> = cats.iter().map(|(s, _)| s).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        assert_eq!(slugs, sorted);
    }
}
