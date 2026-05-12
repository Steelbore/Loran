// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! The validated [`Page`] data structure and its raw deserialisation form.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A fully-validated Loran page.
///
/// Construct via [`Page::parse`](crate::Page::parse); the fields listed
/// here are the post-validation surface, with required fields
/// represented as `String` / `Vec<String>` (never `Option`) and optional
/// metadata represented as `Option<T>`.
///
/// See [`crate`] for the parsing contract.
///
/// `Serialize` + `Deserialize` are implemented so the resolution chain
/// can drop `Page` values straight into the JSON envelope **and** the
/// postcard binary cache (Phase 2 `loran-core::cache`). Callers that
/// want a metadata-only projection (omitting the raw markdown body)
/// build a DTO at their layer rather than asking `Page` to vary its
/// serialised shape — see `loran-cli::cmd::show::ShowData` for the
/// `loran show` envelope and `loran-cli::cmd::list::PageSummary` for
/// the `loran list` array entries.
///
/// **Direct deserialisation bypasses [`Page::parse`]'s schema-validation
/// invariants.** The cache layer only deserialises values written by a
/// previously-validated serialise round-trip; external code paths
/// should still go through `Page::parse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Page {
    /// Canonical binary name, lower-kebab-case (Spec §6.1). Required.
    pub name: String,

    /// Category slug. Slash-tolerant — `system/file-listing` is valid.
    /// Required.
    pub category: String,

    /// One-line description, at most 120 characters. Required.
    pub summary: String,

    /// Broad set of legacy tools this entry supersedes.
    ///
    /// Empty by default. May be non-drop-in: appearing here means
    /// "modern alternative", not "drop-in alias-safe replacement" — the
    /// alias-safe subset lives in [`Page::safe_alias_for`].
    pub replaces: Vec<String>,

    /// Strict subset of [`Page::replaces`] flagging tools the entry can
    /// safely be aliased to without breaking common-case scripts.
    ///
    /// Empty by default. `safe_alias_for ⊆ replaces` is enforced at
    /// parse time.
    pub safe_alias_for: Vec<String>,

    /// Companion tools that work well alongside this one.
    ///
    /// Non-reciprocal — `A.pairs_with = [B]` does not imply
    /// `B.pairs_with = [A]`. Empty by default.
    pub pairs_with: Vec<String>,

    /// Upstream homepage URL. Not validated as a URL in v1.
    pub official: Option<String>,

    /// `tldr-pages` key, or `None` to default to [`Page::name`].
    ///
    /// `Some("")` explicitly disables tldr lookup. Resolution-chain
    /// semantics live in `loran-core`; the parser just stores the raw
    /// value.
    pub tldr_page: Option<String>,

    /// Free-form discovery tags surfaced in `loran search`.
    pub tags: Vec<String>,

    /// Implementation language. Surfaces a 🦀 badge in the TUI when
    /// `Some("rust")`. Free-form otherwise.
    pub written_in: Option<String>,

    /// **Reserved** for future i18n use. Stored verbatim but not
    /// interpreted by v1 consumers (Spec §2 decision #15).
    pub language: Option<String>,

    /// First Steelbore release shipping this tool (e.g. `bravais@0.1`).
    pub since: Option<String>,

    /// Alternative spellings (e.g. `ripgrep` ↔ `rg`).
    pub aliases: Vec<String>,

    /// Everything after the closing `+++` fence, stored verbatim.
    ///
    /// Leading whitespace immediately following the fence is preserved;
    /// downstream renderers handle trimming according to their own
    /// presentation rules.
    ///
    /// **Serialization shape:** the field is serde-`Serialize`d as
    /// `"body"`, which collides with the nested `"body": BodyBlock` in
    /// the `loran show` envelope. Output callers therefore project to
    /// purpose-built DTOs (`ShowData`, `PageSummary`) rather than
    /// flattening a `Page` directly. The postcard cache and any other
    /// consumer that wants the full structured form keeps using `Page`
    /// unmodified.
    pub body: String,
}

/// Raw deserialisation form for the TOML frontmatter block.
///
/// `RawPage` exists solely to give serde a target with `deny_unknown_fields`
/// and `Option`-typed required fields. After deserialisation, we run
/// schema-level validation in `parse::validate` and produce a [`Page`].
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPage {
    pub(crate) name: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) summary: Option<String>,
    #[serde(default)]
    pub(crate) replaces: Vec<String>,
    #[serde(default)]
    pub(crate) safe_alias_for: Vec<String>,
    #[serde(default)]
    pub(crate) pairs_with: Vec<String>,
    pub(crate) official: Option<String>,
    pub(crate) tldr_page: Option<String>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    pub(crate) written_in: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) since: Option<String>,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
}
