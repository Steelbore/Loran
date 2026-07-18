<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# Loran

[![CI](https://github.com/Spacecraft-Software/Loran/actions/workflows/ci.yml/badge.svg)](https://github.com/Spacecraft-Software/Loran/actions/workflows/ci.yml)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](rust-toolchain.toml)
[![crates.io](https://img.shields.io/crates/v/loran.svg)](https://crates.io/crates/loran)

> The Spacecraft Software reference manual — agent-native from day one.

**Loran** is the canonical reference tool for Spacecraft Software-based systems (Bravais, Ferrite OS, future distros). It is to Spacecraft Software what `man` is to Unix and `info` is to GNU — a system-level handbook for every tool the system ships and recommends — with one critical difference: it is agent-native (`--json`, `schema`, stdio MCP) from the first commit.

The name is a heritage engineering acronym: **LO**ng **RA**nge **N**avigation, the radio navigation infrastructure used by ships and aircraft from the 1940s until GPS retired it in 2010. Loran the tool is the precision reference grid for a Spacecraft Software system.

## Overview

Loran answers three questions about the tool catalog of a Spacecraft Software system:

1. **What tools are available here?** — categorised browse of the curated catalog.
2. **What does this tool do, and what does it replace?** — Spacecraft Software-curated intro, with tldr-pages fallback.
3. **What replaces the legacy tool I know?** — reverse lookup (`loran find ls` → `eza`).

A separate verb (`loran help <tool>`) captures live `--help` output from any binary on `$PATH`, rendered in a deliberately de-themed monochrome frame so curated content stays visually distinct from uncurated passthroughs.

## Status

**Released.** Phases 0-3 are feature-complete and tagged.

| Tag | Codename | Scope | Tagged |
|---|---|---|---|
| [`v0.1.0-ingot`](https://github.com/Spacecraft-Software/Loran/releases/tag/v0.1.0-ingot) | Ingot | Text-mode binary; 6 read verbs; bundled catalog. | 2026-05-12 |
| [`v0.2.0-billet`](https://github.com/Spacecraft-Software/Loran/releases/tag/v0.2.0-billet) | Billet | TUI; signed tarball updates; overlays; authoring. The 1.0-equivalent milestone. | 2026-05-12 |
| [`v0.3.0-bloom`](https://github.com/Spacecraft-Software/Loran/releases/tag/v0.3.0-bloom) | Bloom | Read-only MCP; JSON Schema; CLI-Standard describe auto-ingestion; key-rotation primitive. | 2026-05-12 |

Twelve sub-commands functional, 26 curated pages bundled across 10 categories, full ratatui TUI, synchronous stdio JSON-RPC 2.0 MCP server, multi-platform CI matrix green (Linux gnu/musl/aarch64, FreeBSD cross-check, macOS arm64).

## Installation

### From crates.io

```sh
cargo install loran
```

The crate is named `loran`; the installed binary is also `loran`.

### From source

The host's `$CFLAGS` typically carries `-flto=auto`, which corrupts `ring`'s C objects and breaks linking. Build inside a clean gcc env via Nix:

```sh
git clone https://github.com/Spacecraft-Software/Loran
cd Loran
nix shell nixpkgs#gcc -c env -u CFLAGS bash -c 'cargo install --path crates/loran'
```

## Quickstart

```sh
loran                       # launch the TUI when stdout is a TTY
loran list                  # all curated pages, one per row
loran show eza              # curated page for eza
loran find ls               # reverse lookup: what replaces ls?
loran search json           # fuzzy search across name / summary / replaces / tags
loran help rg               # live --help capture (de-themed monochrome frame)
loran schema --json         # emit Draft 2020-12 JSON Schema for the data model
loran categories            # category registry with counts
```

Add `--json` to any read verb to get a structured envelope (per the CLI Standard §6). Set `AI_AGENT=1` or `AGENT=1` in the environment and the TUI never activates — Loran emits JSON or a usage hint instead.

## Features

- **Two distinct verbs by design.** `loran show` is curated-or-fail (Spacecraft Software palette + intro block); `loran help` is always-live `--help` capture in a deliberately de-themed monochrome frame. Brand cues stay reserved for curated content.
- **Three-layer overlays** with field-by-field merge: `upstream < distro < user`. Distro layer resolves from `/etc/os-release` (`ID=bravais`, `ID=ferrite`, …) and is overridable with `--overlay <name>` or `LORAN_DISTRO_OVERRIDE`.
- **Signed tarball updates** — minisign + ed25519, trust-pinned at build time, parallel-key rotation via `loran-core::signing::verify_any` (see [`OPERATIONS.md`](OPERATIONS.md)).
- **Agent-native by default** — `--json` on every read verb, `loran schema` emits Draft 2020-12 JSON Schema for every public type, `loran mcp` exposes a read-only MCP stdio server with the five read verbs.
- **No tokio anywhere.** Fully synchronous workspace, including the MCP server — one in-flight RPC at a time, blocking stdin reads.
- **TUI** built on `ratatui` + `crossterm` — browse view (categories ↔ tools), detail view (rendered / raw / frontmatter sub-views), `/`-fuzzy search, `?`-help overlay, panic-safe terminal restoration.
- **`DescribeIngestor`** — auto-synthesises pages from any CLI-Standard-compliant tool's `<tool> describe --json` output (allowlisted via `LORAN_DESCRIBE_BINARIES`).
- **Hermetic test seams** — every integration test sandboxes via env vars (`XDG_DATA_HOME`, `LORAN_DISTRO_OVERRIDE`, `LORAN_PAGES_*_URL`, …). No test ever writes to the real home.

## CLI reference

| Verb | Synopsis | Type | Notes |
|---|---|---|---|
| `list` | `loran list [--category C] [--replaces L] [--safe-alias-for L]` | Curated | Catalog index. |
| `show` | `loran show <tool>` | Curated | Curated-or-fail. Never falls through to live `--help`. |
| `find` | `loran find <legacy> [--safe-alias]` | Curated | Reverse lookup. |
| `search` | `loran search <query>` | Curated | Fuzzy match across name, summary, replaces, tags. |
| `categories` | `loran categories` | Curated | Registry with counts. |
| `help` | `loran help <tool> [--pager CMD]` | Live | De-themed `--help` capture from `$PATH`. |
| `new` | `loran new <tool> [--category C] [--replaces L...] ...` | Write | Interactive prompt if no flags. |
| `update` | `loran update` | Write | Refreshes upstream + tldr tarballs over HTTPS, verifies minisign. |
| `validate` | `loran validate` | Meta | Walks every overlay root; emits structured errors per file. |
| `schema` | `loran schema [--json]` | Meta | Draft 2020-12 JSON Schema for every public type. |
| `describe` | `loran describe [--json]` | Meta | The CLI Standard's describe manifest for agents. |
| `mcp` | `loran mcp` | Meta | Stdio JSON-RPC 2.0 MCP server (read-only verbs only). |
| _(default)_ | `loran` (no verb) | TUI | ratatui app when stdout is a TTY and no agent env detected. |

## Bundled content

Loran v0.3.0 ships **26 curated pages across 10 categories**:

- **data-processing** (4): dasel, gron, jaq, miller
- **file-listing** (3): broot, eza, lsd
- **file-search** (2): dust, fd
- **file-viewing** (3): bat, delta, hexyl
- **networking** (2): dog, xh
- **process-management** (1): procs
- **shell-utilities** (5): direnv, hyperfine, just, starship, zoxide
- **system-monitoring** (2): bandwhich, bottom
- **text-search** (2): rg, sd
- **version-control** (2): git-cliff, lazygit

Categories are first-class structural keys defined in `crates/loran-core/pages/categories.toml`. The category slug is slash-tolerant (nested UX is deferred to the ~50-entry threshold per Spec §15). Authoring a new page locally is one command: `loran new <tool> --category=<slug> --summary="…"`.

## Architecture

Eight crates plus `xtask`. Read top-to-bottom — each layer depends on the one above.

| Crate | Purpose |
|---|---|
| `loran-pages` | Page parser (TOML frontmatter + Markdown). `Page::parse` + `OverlayPage::parse` for partial-frontmatter layered merges. |
| `loran-index` | `Ingestor` trait, `MarkdownPagesIngestor`, `LayeredIngestor` (upstream < distro < user precedence), `DescribeIngestor` (auto-synthesise pages from `<tool> describe --json`), `/etc/os-release` distro detection. |
| `loran-core` | Resolution chains (`resolve_show` / `resolve_find` / `resolve_search`), fetch / extract / minisign-verify pipelines, tldr cache, `update_pages` / `update_tldr`, schemars-derived public types, `xdg::data_home()` / `cache_home()` helpers. Bundled curated pages live under `crates/loran-core/pages/`. |
| `loran-render` | Markdown → plain-text renderer for stdout / pager. |
| `loran-tui` | ratatui app: browse view (categories + tools), detail view (rendered / raw / frontmatter sub-views), search overlay, in-app help overlay, `loran new` interactive prompt. |
| `loran-mcp` | Synchronous stdio JSON-RPC 2.0 MCP server. Five read-only verbs only (`list` / `show` / `find` / `search` / `categories`); write verbs and `help` rejected with `WRITE_VERB_REJECTED` (-32001). |
| `loran` | Binary `loran`. Sub-command handlers under `src/cmd/`; `src/index_loader.rs::build_layered_index_with_overlay()` is the shared layered-index builder every read verb threads through. |
| `loran-tldr` | Placeholder — the live logic was absorbed into `loran-core::tldr` in Phase 2. Not published. |
| `xtask` | Workspace tooling. `cargo xtask check-spdx` walks every text file for the SPDX header. |

## Building from source

The pre-commit gate — equivalent to what `tier1 (linux-gnu)` runs in CI:

```sh
nix shell nixpkgs#gcc -c env -u CFLAGS RUSTFLAGS='-D warnings' bash -c '
  cargo fmt --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo xtask check-spdx
'
```

Cargo audit (binary isn't in the workspace):

```sh
nix shell nixpkgs#cargo-audit -c env -u CFLAGS bash -c 'cargo audit --deny warnings'
```

Regenerate insta snapshots after intentional output changes:

```sh
INSTA_UPDATE=always cargo test -p loran --test snapshots
```

## Platform support

| Tier | Target | Status |
|---|---|---|
| 1 | `x86_64-unknown-linux-gnu` | Primary developer target. Full CI: fmt, clippy `-D warnings`, test, NFR thresholds in release, cargo audit, SPDX check. |
| 1 | `x86_64-unknown-linux-musl` | Static-binary CI lane. Build + test. |
| 1 | `aarch64-unknown-linux-gnu` | Native ARM CI runner. Build + test. |
| 1 | `x86_64-unknown-freebsd` | Cross-`check` only (no sysroot — catches cfg drift at the type-checker). |
| 2 | `aarch64-apple-darwin` | Informational; macOS XDG handled via `loran_core::data_home()` / `cache_home()` which honour `XDG_DATA_HOME` / `XDG_CACHE_HOME` on every platform. |
| — | Windows | Not supported (no current need; codebase is XDG-first). |

## Configuration

Loran follows the XDG Base Directory Specification on every platform, including macOS (where `dirs::data_dir()` would otherwise return `~/Library/Application Support`).

```text
$XDG_DATA_HOME/loran/
├── pages/                    # upstream pages, synced via `loran update`
├── overlays/
│   ├── <active-distro>/      # distro overlay (e.g. bravais, ferrite, generic)
│   └── user/                 # user overlay — highest precedence
├── sources.toml              # manifest pointers (publisher URL, public key, …)
└── tldr.zip                  # tldr-pages archive (cached after first update)

$XDG_CACHE_HOME/loran/
├── index.postcard            # binary index cache (rebuilt automatically)
└── tldr/                     # extracted tldr-pages per platform
```

## Design decisions

All key design decisions are locked in [`loran-spec-v0_2.md`](loran-spec-v0_2.md) §2. Highlights:

- Greenfield workspace; no fork of tealdeer or tlrc (reuse patterns, not code).
- Curated-or-fail `show` vs always-live `help` (verb split).
- TOML frontmatter fenced by `+++` (Hugo/Zola style). `safe_alias_for ⊆ replaces` is a hard validation invariant.
- Tarball + minisign update model; no runtime git client.
- No tokio anywhere; MCP is synchronous stdio.
- MCP surface is read-only — write verbs and `help` rejected with `WRITE_VERB_REJECTED` (-32001).

## Open spec questions

From [`loran-spec-v0_2.md`](loran-spec-v0_2.md) §15:

1. **`pairs_with` reciprocity** — should `loran validate` warn when A claims `pairs_with = ["B"]` but B does not reciprocate, or accept asymmetry as intentional?
2. **`DescribeIngestor` security model** — allowlist (current default: env-driven via `LORAN_DESCRIBE_BINARIES`) vs self-declaration via the CLI Standard's `describe`?

## Contributing

Contributions are welcome but acceptance is at the maintainer's discretion (Standard §5.4). Quick checklist:

- DCO sign-off (`git commit -s …`).
- SPDX header on every text file (`xtask check-spdx` enforces this).
- Pre-commit gate green (see [Building from source](#building-from-source)).
- See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the full guide.

Agents working on Loran should read [`AGENTS.md`](AGENTS.md) (tool-agnostic), [`CLAUDE.md`](CLAUDE.md) (Claude Code-specific), and [`SKILL.md`](SKILL.md) (Spacecraft Software skill surface). The four governing documents (`loran-{prd,spec,plan,todo}-v0_X.md`) are canonical.

## License & posture

Loran is a **personal hobby project** under Spacecraft Software. Per Spacecraft Software Standard v1.1 §5.1:

| Aspect         | Stance                                                         |
|----------------|----------------------------------------------------------------|
| Audience       | Maintainer's own use case                                      |
| Pace           | Hobby pace; no service-level commitments                       |
| Warranty       | None — provided AS IS (see [NOTICE](NOTICE.md))                |
| Liability      | None (see [NOTICE](NOTICE.md))                                 |
| Contributions  | Welcome but not guaranteed to be accepted (see [CONTRIBUTING](CONTRIBUTING.md)) |
| Forking        | Encouraged                                                     |
| License        | GPL-3.0-or-later (see [LICENSE](LICENSE) — formal terms govern in any conflict) |

The PR-acceptance bar, feature scope, naming, and roadmap are at the maintainer's sole discretion (Standard §5.4).

Loran is licensed under [GPL-3.0-or-later](LICENSE). SPDX headers on every source file. See [`NOTICE.md`](NOTICE.md) for the warranty / liability statement.

## Maintainer

**Mohamed Hammad** &lt;Mohamed.Hammad@SpacecraftSoftware.org&gt;

- Project: <https://Loran.SpacecraftSoftware.org/>
- Repository: <https://github.com/Spacecraft-Software/Loran>
- Issues: <https://github.com/Spacecraft-Software/Loran/issues>
- Crates: <https://crates.io/crates/loran>

Copyright © 2026 Mohamed Hammad.

---

*Forged in Spacecraft Software.*
