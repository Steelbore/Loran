// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Mohamed Hammad

//! `clap`-derived top-level [`Cli`] type plus the [`Command`] enum and
//! its per-sub-command arg bundles.

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Project attribution shown in `--version` and `--help` footers per
/// Spacecraft Software Standard v1.1 §13.2.
pub(crate) const ATTRIBUTION_FOOTER: &str = "\
Maintained by Mohamed Hammad <Mohamed.Hammad@SpacecraftSoftware.org>
Project: https://Loran.SpacecraftSoftware.org/
Source: https://github.com/Spacecraft-Software/Loran";

/// Output format for sub-commands that emit structured data.
///
/// `Human` is the TTY-friendly default; `Json` produces the SFRS §6
/// envelope. The `--json` global flag is sugar for `--format json`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum Format {
    #[default]
    Human,
    Json,
}

/// Color-mode override.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

/// SFRS §3 global flags. Every flag is `global = true` so it can be
/// supplied either before or after the sub-command — `loran --json list`
/// and `loran list --json` behave identically.
#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)] // SFRS §3 dictates the flag shape
pub(crate) struct GlobalFlags {
    /// Emit machine-readable JSON output. Equivalent to `--format json`.
    #[arg(long, global = true)]
    pub json: bool,

    /// Output format. Sugar: `--json` ⇔ `--format json`.
    #[arg(long, value_enum, value_name = "FORMAT", global = true)]
    pub format: Option<Format>,

    /// Restrict structured output to a comma-separated subset of fields.
    #[arg(long, value_name = "LIST", global = true, value_delimiter = ',')]
    pub fields: Vec<String>,

    /// Plan but do not perform side effects.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Increase log verbosity. May be repeated (`-vv` for trace).
    #[arg(long, short, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all non-error output.
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Color mode. Defaults to `auto` (respects `NO_COLOR`, TTY).
    #[arg(long, value_enum, value_name = "WHEN", global = true)]
    pub color: Option<ColorMode>,

    /// Force-disable ANSI color. Alias for `--color=never`. Honours `NO_COLOR`.
    #[arg(long, global = true, conflicts_with = "color")]
    pub no_color: bool,

    /// Render timestamps as absolute ISO 8601 UTC.
    #[arg(long, global = true)]
    pub absolute_time: bool,

    /// Use NUL separators in list-style output (for `xargs -0`).
    #[arg(long, global = true)]
    pub print0: bool,

    /// Assume "yes" to confirmation prompts.
    #[arg(long, global = true)]
    pub yes: bool,

    /// Suppress all network access for this invocation: disables the
    /// opt-in catalog auto-update. (`loran update` still errors loudly
    /// rather than silently succeeding.)
    #[arg(long, global = true)]
    pub offline: bool,

    /// Pin the active distro overlay layer by name (e.g. `bravais`,
    /// `ferrite`). Highest-precedence override — beats
    /// `LORAN_DISTRO_OVERRIDE` and `/etc/os-release`. Useful for
    /// previewing another distro's curation from a different host.
    #[arg(long, value_name = "NAME", global = true)]
    pub overlay: Option<String>,
}

/// Top-level CLI surface.
///
/// `disable_version_flag = true` because we surface `--version` ourselves
/// — we need the flag to interact with `--json` (printing the SFRS
/// envelope) and `--format`, which clap's auto-version handling doesn't
/// support. See [`crate::version::emit`].
#[derive(Debug, Parser)]
#[command(
    name = "loran",
    version = env!("CARGO_PKG_VERSION"),
    about = "Loran — the Spacecraft Software reference manual.",
    long_about = "Loran is the canonical, agent-friendly reference tool for \
        Spacecraft Software-based systems. Browse the curated tool catalog, look up what \
        replaces a legacy tool, or capture a binary's live --help output.",
    disable_version_flag = true,
    disable_help_subcommand = true,
    after_help = ATTRIBUTION_FOOTER,
    after_long_help = ATTRIBUTION_FOOTER,
)]
pub(crate) struct Cli {
    /// Print version info and exit. Use with `--json` for the SFRS envelope.
    #[arg(long, short = 'V', global = true)]
    pub version: bool,

    #[command(flatten)]
    pub global: GlobalFlags,

    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    /// Resolved output format — explicit `--format` wins over the
    /// `--json` boolean sugar.
    pub(crate) fn output_format(&self) -> Format {
        if let Some(fmt) = self.global.format {
            fmt
        } else if self.global.json {
            Format::Json
        } else {
            Format::Human
        }
    }
}

/// The twelve Loran sub-commands per Spec §7.
///
/// Sub-command-specific flags follow each variant's struct. Phase 1B
/// declares the surface in full so `loran <verb> --help` already lists
/// every accepted flag even before handlers are implemented; that gives
/// agents an authoritative `loran describe`-equivalent the moment the
/// binary ships, ahead of the formal `describe` sub-command landing in
/// Sub-phase 1C.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// List tools in the catalog.
    List(ListArgs),
    /// Show a curated page (curated-or-fail; never falls through to live --help).
    Show(ShowArgs),
    /// Capture and render a binary's --help output directly (de-themed).
    Help(HelpArgs),
    /// Reverse lookup: which tool replaces a legacy name?
    Find(FindArgs),
    /// Fuzzy search across name, summary, replaces, and tags.
    Search(SearchArgs),
    /// List categories with counts.
    Categories(CategoriesArgs),
    /// Scaffold a new curated page.
    New(NewArgs),
    /// Refresh upstream tarballs (signed) and rebuild the index.
    Update(UpdateArgs),
    /// Validate every page against the frontmatter schema.
    Validate(ValidateArgs),
    /// Emit JSON Schema for the Loran data model.
    Schema(SchemaArgs),
    /// Emit the SFRS describe manifest for agents.
    Describe(DescribeArgs),
    /// Run as a read-only MCP server over stdio.
    Mcp(McpArgs),
}

// ─── Per-sub-command arg bundles ─────────────────────────────────────

#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    /// Filter to pages in this category.
    #[arg(long, value_name = "CATEGORY")]
    pub category: Option<String>,

    /// Filter to pages that supersede this legacy tool.
    #[arg(long, value_name = "LEGACY")]
    pub replaces: Option<String>,

    /// Filter to pages whose `safe_alias_for` includes this legacy tool.
    #[arg(long, value_name = "LEGACY")]
    pub safe_alias_for: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct ShowArgs {
    /// Tool name (canonical, lower-kebab-case).
    pub tool: String,
}

#[derive(Debug, Args)]
pub(crate) struct HelpArgs {
    /// Tool name; must resolve via `$PATH`.
    pub tool: String,

    /// Override the §4.2.1 pager cascade.
    ///
    /// Reserved sentinels: `--pager=""` disables pagination
    /// (cat-equivalent passthrough); `--pager=loran` forces the
    /// Spacecraft Software default chain (`bat -pp` → `moor` → `cat`) and skips
    /// the user's `$MANPAGER` / `$PAGER` environment.
    #[arg(long, value_name = "CMD")]
    pub pager: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct FindArgs {
    /// Legacy tool name to reverse-lookup.
    pub legacy: String,

    /// Restrict matches to entries that declare `<legacy>` in
    /// `safe_alias_for` (not just `replaces`).
    #[arg(long)]
    pub safe_alias: bool,
}

#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    /// Free-text query, fuzzy-matched against name / summary / replaces / tags.
    pub query: String,
}

#[derive(Debug, Args)]
pub(crate) struct CategoriesArgs {}

#[derive(Debug, Args)]
pub(crate) struct NewArgs {
    /// Tool name (canonical, lower-kebab-case).
    pub tool: String,

    /// Category slug (slash-tolerant).
    #[arg(long)]
    pub category: Option<String>,

    /// Comma-separated list of legacy tools this entry supersedes.
    #[arg(long, value_delimiter = ',')]
    pub replaces: Vec<String>,

    /// Strict subset of `--replaces` flagging alias-safe legacy tools.
    #[arg(long, value_delimiter = ',')]
    pub safe_alias_for: Vec<String>,

    /// One-line summary (≤120 chars).
    #[arg(long)]
    pub summary: Option<String>,

    /// Open `$EDITOR` on the body after scaffolding.
    #[arg(long, conflicts_with = "no_edit")]
    pub edit: bool,

    /// Suppress the editor open (for non-interactive use).
    #[arg(long)]
    pub no_edit: bool,

    /// Overlay scope to write into.
    #[arg(long, value_enum, value_name = "SCOPE", default_value = "user")]
    pub scope: NewScope,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum NewScope {
    #[default]
    User,
    Upstream,
}

#[derive(Debug, Args)]
pub(crate) struct UpdateArgs {
    /// Refuse to fetch any tarball whose publisher does not sign with
    /// minisign + ed25519. The Loran upstream signs; tldr-pages does not.
    #[arg(long)]
    pub require_signatures: bool,

    /// Ignore the cached `ETag` and re-fetch unconditionally.
    #[arg(long)]
    pub force_refresh: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// Validate this directory as an upstream-strict pages tree (full
    /// pages only) instead of the on-disk overlay roots under
    /// `$XDG_DATA_HOME/loran/`. Intended for CI gating a pages repo,
    /// e.g. `loran validate pages/`.
    #[arg(value_name = "ROOT")]
    pub root: Option<std::path::PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct SchemaArgs {
    /// Optional sub-key to emit a partial schema for (e.g. `page`).
    pub key: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct DescribeArgs {}

#[derive(Debug, Args)]
pub(crate) struct McpArgs {}
