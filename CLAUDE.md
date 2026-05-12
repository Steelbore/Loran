<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# CLAUDE.md

This file provides Claude-Code-specific guidance for working with the Loran repository. For coding conventions, forbidden patterns, the pre-commit gate, repository layout, and CLI conformance rules, **read [AGENTS.md](AGENTS.md) first** — those are tool-agnostic and apply to every coding agent. This file only adds what's specific to Claude Code (skills system, planning-doc reading order, locked design decisions worth knowing before you start).

## Repository status

Phases 0–3 are feature-complete and tagged: `v0.1.0-ingot` (Phase 1), `v0.2.0-billet` (Phase 2), `v0.3.0-bloom` (Phase 3). All 12 sub-commands functional, 25 bundled curated pages, full TUI, MCP stdio server, multi-platform CI matrix green (Linux gnu/musl/aarch64, FreeBSD cross-check, macOS arm64). `loran-todo-v0_1.md` still carries many unchecked Phase-0/1/2/3 entries — most are paperwork debt; the work is done. New feature WPs would be Phase 4+ and don't exist in the plan yet.

## Common commands

The host's `$CFLAGS` typically carries `-flto=auto`, which corrupts `ring`'s C objects and breaks linking. Every cargo invocation should run inside a clean gcc env via Nix:

```sh
nix shell nixpkgs#gcc -c env -u CFLAGS bash -c '<cargo …>'
```

The pre-commit gate — equivalent to what `tier1 (linux-gnu)` runs in CI:

```sh
nix shell nixpkgs#gcc -c env -u CFLAGS RUSTFLAGS='-D warnings' bash -c '
  cargo fmt --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo xtask check-spdx
'
```

Single test:

```sh
nix shell nixpkgs#gcc -c env -u CFLAGS bash -c 'cargo test -p loran-cli --test mcp mcp_responds_to_initialize'
```

Cargo audit (provide the binary separately — it's not in the workspace):

```sh
nix shell nixpkgs#cargo-audit -c env -u CFLAGS bash -c 'cargo audit --deny warnings'
```

Regenerate insta snapshots after intentional output changes:

```sh
INSTA_UPDATE=always cargo test -p loran-cli --test snapshots
```

## Architecture at a glance

Eight crates plus `xtask`. Read top-to-bottom — each layer depends on the one above.

| Crate | Purpose |
|---|---|
| `loran-pages` | Page parser (TOML frontmatter + Markdown). `Page::parse` + `OverlayPage::parse` for partial-frontmatter layered merges. |
| `loran-index` | `Ingestor` trait, `MarkdownPagesIngestor`, `LayeredIngestor` (upstream < distro < user precedence), `DescribeIngestor` (auto-synthesise pages from `<tool> describe --json`), `/etc/os-release` distro detection. |
| `loran-core` | Resolution chains (`resolve_show` / `resolve_find` / `resolve_search`), fetch / extract / minisign-verify pipelines, tldr cache, `update_pages` / `update_tldr`, schemars-derived public types, `xdg::data_home()` / `cache_home()` helpers. |
| `loran-render` | Markdown → plain-text renderer for stdout / pager. |
| `loran-tui` | ratatui app: browse view (categories + tools), detail view (rendered / raw / frontmatter sub-views), search overlay, in-app help overlay, `loran new` interactive prompt. |
| `loran-mcp` | Synchronous stdio JSON-RPC 2.0 MCP server. Five read-only verbs only (`list` / `show` / `find` / `search` / `categories`); write verbs and `help` rejected with `WRITE_VERB_REJECTED` (-32001). |
| `loran-cli` | Binary `loran`. Sub-command handlers under `src/cmd/`; `src/index_loader.rs::build_layered_index_with_overlay()` is the shared layered-index builder every read verb threads through. |
| `loran-tldr` | Placeholder — the live logic was absorbed into `loran-core::tldr` in Phase 2. |
| `xtask` | Workspace tooling. `cargo xtask check-spdx` walks every text file for the SPDX header. |

## The four governing documents (read in this order)

1. **`loran-prd-v0_1.md`** — *what* Loran must accomplish. Goals (G-01…G-10), quality goals (Q-01…Q-07), non-goals, user stories, functional + non-functional requirements.
2. **`loran-spec-v0_2.md`** — *how* Loran will be built. Canonical technical reference: workspace layout, dependency stack, resolution chains for `show`/`help`, page frontmatter schema, JSON envelope, exit codes, filesystem layout, tarball update mechanism. **Spec v0.2 supersedes the older `lodestone-spec-v0.1.md`** — that file describes the project under its previous name (Lodestone) and is kept only as historical context.
3. **`loran-plan-v0_1.md`** — work packages with sizing, dependencies, and acceptance criteria. Operationalises the PRD against the spec.
4. **`loran-todo-v0_1.md`** — task-level decomposition with stable IDs (`LOR-PXXX-NNN`).

Operational reference, post-Phase 3:

- **`OPERATIONS.md`** — minisign key rotation runbook (normal annual cadence + emergency-compromise procedure). Resolved PRD Open Question 1.

When the user references "the spec", "the PRD", "the plan", or "the TODO" without qualification, they mean these v0.x documents. The `.md` files are canonical — the `.odt` and `.md.pdf` siblings are exported views.

## Project renames since v0.1

Both apply across all current docs except the retained `lodestone-spec-v0.1.md`:

- **Lodestone → Loran.** LORAN (LOng RAnge Navigation), the radio navigation system retired by GPS in 2010.
- **Lattice → Bravais.** Bravais refers to the 14 Bravais lattices in crystallography. Overlay paths and the `/etc/os-release` ID all reflect this.

If you find "Lodestone" or "Lattice" outside the historical v0.1 spec, treat it as a stale reference to update.

## Steelbore skills to load

Before generating or reviewing any artifact, consult these skills via the Skill tool. They encode constraints that the Loran documents reference rather than restate.

- **`steelbore-standard`** — Standard v1.1 (palette, posture files, attribution, ISO 8601 UTC with `Z`, GPL-3.0-or-later + SPDX headers).
- **`steelbore-cli-standard`** — SFRS v1.0.0: noun-verb commands, global flag set, `--json` / `--format` / `schema` / `describe`, exit-code conventions, JSON envelope shape, structured errors to stderr.
- **`steelbore-agentic-cli`** — agent-facing UX: TTY auto-detect cascade, `AI_AGENT`/`AGENT` env vars suppressing TUI, tips-thinking error hints, lazy MCP schema loading, AGENTS.md/CLAUDE.md/SKILL.md conventions.
- **`rust-guidelines`** — Microsoft Pragmatic Rust Guidelines. Mandatory before writing any `.rs` file.
- **`steelbore-cli-preference`** — substitute modern CLI tools (`eza`, `bat`, `rg`, `fd`, `jaq`, `xh`, `dog`, `procs`, `bottom`, …) for legacy ones in any shell command you run, suggest, or write into a script.
- **`steelbore-cli-shell`** — shell-syntax compliance (POSIX first; Nushell / Ion / PowerShell / ash; no Bash-isms in portable scripts). Target shells: Nushell and Ion; POSIX `sh` in scripts.
- **`steelbore-missing-pkg`** — package-manager priority (Guix → Nix → Cargo → Homebrew → Flatpak → Snap). Never apt/dnf/pacman.

## Locked design decisions

From Spec §2 — challenging these is out of scope without an explicit user decision to revisit:

- Greenfield Cargo workspace; no fork of tealdeer or tlrc (reuse patterns, not code).
- Two distinct verbs with different brand semantics: **`loran show`** is curated-or-fail (no live `--help` fallback); **`loran help`** is always-live `--help` capture, rendered in a deliberately **de-themed monochrome frame** so the Steelbore palette stays reserved for curated content.
- Pages: single Markdown file with **TOML frontmatter fenced by `+++`** (Hugo/Zola style). `safe_alias_for ⊆ replaces` is a hard validation invariant.
- Frontmatter `written_in` is implementation language; **`language` is reserved for future i18n** (per tldr-pages `pages.<lang>/` precedent).
- Three overlay layers in precedence order: upstream `pages/` < `overlays/<active-distro>/` (Bravais or Ferrite OS, resolved from `/etc/os-release`) < `overlays/user/`. Merge is field-by-field, not record-by-record.
- Tarball update model (no runtime git dependency), **minisign + ed25519** signatures verified against a publisher key baked into the binary via `include_str!`. Key rotation requires a new Loran release; the parallel-key transition primitive is `loran-core::signing::verify_any` and is documented in `OPERATIONS.md`.
- **No `tokio` anywhere.** The MCP server is synchronous stdio — one in-flight RPC at a time, blocking reads on stdin. Async is not used in the workspace.
- **MCP surface is read-only** (`list`, `show`, `find`, `search`, `categories`). `update`, `new`, `validate`, and `help` are deliberately rejected with `WRITE_VERB_REJECTED` (-32001).
- TUI never activates when `AI_AGENT=1` or `AGENT=1` is set.
- An **`Ingestor` trait** in `loran-index` is the single seam for new page sources. `MarkdownPagesIngestor`, `LayeredIngestor`, and `DescribeIngestor` all flow through it.

## Watch out for

- **macOS XDG override.** `dirs::data_dir()` and `dirs::cache_dir()` return `~/Library/Application Support` / `~/Library/Caches` on macOS and **do not honour** `$XDG_DATA_HOME` / `$XDG_CACHE_HOME`. New code that needs the data or cache root must go through `loran_core::data_home()` / `loran_core::cache_home()`, which honour the env vars first across every platform. Calling `dirs::*_dir()` directly will silently work on Linux but corrupt the macOS test runner's home.

- **Hermetic integration tests.** Every integration test sandboxes via env vars rather than touching the host filesystem. The seams:
  - `XDG_DATA_HOME` / `XDG_CACHE_HOME` — overlay roots, sources.toml, tldr cache.
  - `LORAN_DISTRO_OVERRIDE` — pin the distro layer name without touching `/etc/os-release`.
  - `LORAN_DESCRIBE_BINARIES` — comma-separated allowlist for `DescribeIngestor`.
  - `LORAN_PAGES_MANIFEST_URL` / `_TARBALL_URL` / `_SIG_URL` / `_PUBLIC_KEY` — point `loran update` at a staging publisher (or `http://127.0.0.1:1/` for connection-refused tests).
  - `LORAN_TLDR_ARCHIVE_URL` — tldr archive override.
  - `LORAN_UPSTREAM_PATH` — `loran new --scope=upstream` target.

  New integration tests should follow the same convention rather than write to the real home.

- **Signed commits.** `commit.gpgsign = true` with SSH signing is on this host. `git log --show-signature` falsely reports "No signature" because `gpg.ssh.allowedSignersFile` isn't set locally; the commit object still carries `gpgsig -----BEGIN SSH SIGNATURE-----` and GitHub validates against the registered SSH signing key (commits show as **Verified** on the web UI). Don't try to "fix" the missing local signature — the verification path is fine.

- **CI matrix.** Five entries: linux-gnu (full pipeline), linux-musl, linux-aarch64 (native ARM runner), freebsd-build (cross-`check`, no sysroot), macos-arm64 (informational tier 2). New platform-touching code must compile on all four code paths; the FreeBSD entry catches cfg drift at the type-checker without invoking the linker.

## Don't

- Don't run `git init`, `cargo init`, or create posture files in a session that wasn't explicitly authorized for that work — those are tracked tasks (`LOR-P000-001` etc.).
- Don't add `tokio` or any async runtime — the workspace is synchronous everywhere, including the MCP server.
- Don't apply the Steelbore palette to `loran help` capture output (Spec §2 decision #11).
- Don't expose write-side verbs (`update`, `new`, `validate`) or `help` over the MCP surface.
- Don't call `dirs::data_dir()` / `dirs::cache_dir()` directly — use `loran_core::data_home()` / `cache_home()`.
- Don't fork tealdeer / tlrc — reuse patterns only.
- Don't use `apt`, `dnf`, `pacman`, or other system-distro package managers. Use Guix/Nix/Cargo/Homebrew/Flatpak/Snap per `steelbore-missing-pkg`.
- Don't bypass the DCO sign-off requirement or skip the pre-commit gate (see "Common commands" above).
