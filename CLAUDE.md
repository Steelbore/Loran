# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository status: pre-implementation

This directory contains **planning documents only**. There is no source code, no `Cargo.toml`, no git repository — just four governing documents (and their ODT/PDF renders). Phase 0 (workspace scaffolding) has not yet started. Do not assume crates, build commands, or directories from the spec exist on disk until you have verified with `ls`.

When implementation begins, it will land here as a Cargo workspace with eight crates (see Spec §3.1). The first commits will be Phase 0 (`WP-P0.01` through `WP-P0.05`) — repo init, posture files, agent context files, workspace skeleton, CI.

## The four governing documents (read in this order)

1. **`loran-prd-v0_1.md`** — *what* Loran must accomplish. Goals (G-01…G-10), quality goals (Q-01…Q-07), non-goals, user stories, functional + non-functional requirements. Read first for orientation.
2. **`loran-spec-v0_2.md`** — *how* Loran will be built. Canonical technical reference: workspace layout, dependency stack, resolution chains for `show`/`help`, page frontmatter schema, JSON envelope, exit codes, filesystem layout, tarball update mechanism. **Spec v0.2 supersedes the older `lodestone-spec-v0.1.md`** — that file describes the project under its previous name (Lodestone) and is kept only as historical context.
3. **`loran-plan-v0_1.md`** — work packages with sizing, dependencies, and acceptance criteria. Operationalises the PRD against the spec.
4. **`loran-todo-v0_1.md`** — task-level decomposition with stable IDs (`LOR-PXXX-NNN`). Phase 0 and Phase 1 are detailed; Phase 2/3 are stubbed until earlier phases ship.

When the user references "the spec", "the PRD", "the plan", or "the TODO" without qualification, they mean these v0.x documents. The `.md` files are canonical — the `.odt` and `.md.pdf` siblings are exported views.

## Project renames since v0.1

Both apply across all current docs except the retained `lodestone-spec-v0.1.md`:

- **Lodestone → Loran.** LORAN (LOng RAnge Navigation), the radio navigation system retired by GPS in 2010. Navigation-as-reference metaphor preserved.
- **Lattice → Bravais.** Bravais refers to the 14 Bravais lattices in crystallography. Overlay paths, `/etc/os-release` ID, and the symlink `/steelbore/lattice -> /steelbore/bravais` all reflect this.

If you find "Lodestone" or "Lattice" outside the historical v0.1 spec, treat it as a stale reference to update.

## Steelbore governance — skills to load

Loran is a Steelbore-umbrella project, so **before generating or reviewing any artifact**, consult these skills via the Skill tool:

- **`steelbore-standard`** — Steelbore Standard v1.1 (palette, posture files, attribution, ISO 8601 UTC with `Z`, GPL-3.0-or-later + SPDX headers, metallurgical naming with the codename exception granted to Loran).
- **`steelbore-cli-standard`** — SFRS v1.0.0: noun-verb commands, global flag set, `--json` / `--format` / `schema` / `describe`, exit-code conventions, JSON envelope shape, structured errors to stderr.
- **`steelbore-agentic-cli`** — agent-facing UX: TTY auto-detect cascade, `AI_AGENT`/`AGENT` env vars suppressing TUI, tips-thinking error hints, lazy MCP schema loading, AGENTS.md/CLAUDE.md/SKILL.md conventions.
- **`rust-guidelines`** — Microsoft Pragmatic Rust Guidelines. Mandatory before writing any `.rs` file.
- **`steelbore-cli-preference`** — substitute modern CLI tools (`eza`, `bat`, `rg`, `fd`, `jaq`, `xh`, `dog`, `procs`, `bottom`, …) for legacy ones in any shell command you run, suggest, or write into a script.
- **`steelbore-cli-shell`** — shell-syntax compliance (POSIX first; Nushell / Ion / PowerShell / ash; no Bash-isms in portable scripts). Loran's target shells are Nushell and Ion; POSIX `sh` in scripts.
- **`steelbore-missing-pkg`** — package-manager priority (Guix → Nix → Cargo → Homebrew → Flatpak → Snap). Never apt/dnf/pacman.

These skills encode constraints that are referenced — not restated — in the Loran documents. Read them rather than guessing.

## Locked design decisions worth keeping in mind

From Spec §2 — challenging these is out of scope without an explicit user decision to revisit:

- Greenfield Cargo workspace; no fork of tealdeer or tlrc (reuse patterns, not code).
- Two distinct verbs with different brand semantics: **`loran show`** is curated-or-fail (no live `--help` fallback); **`loran help`** is always-live `--help` capture, rendered in a deliberately **de-themed monochrome frame** so the Steelbore palette stays reserved for curated content.
- Pages: single Markdown file with **TOML frontmatter fenced by `+++`** (Hugo/Zola style). `safe_alias_for ⊆ replaces` is a hard validation invariant.
- Frontmatter `written_in` is implementation language; **`language` is reserved for future i18n** (per tldr-pages `pages.<lang>/` precedent).
- Three overlay layers in precedence order: upstream `pages/` < `overlays/<active-distro>/` (Bravais or Ferrite OS, resolved from `/etc/os-release`) < `overlays/user/`. Merge is field-by-field, not record-by-record.
- Tarball update model (no runtime git dependency), **minisign + ed25519** signatures verified against a publisher key baked into the binary via `include_bytes!`. Key rotation requires a new Loran release. New exit code: `TARBALL_VERIFY_FAILED = 11`.
- **No `tokio` on the v1 fast path.** Tarball fetch is one synchronous `ureq` request. Async is confined to the Phase 3 `loran-mcp` crate.
- **MCP surface is read-only** (`list`, `show`, `find`, `search`, `categories`). `update`, `new`, `validate`, and `help` are deliberately not exposed to agents.
- TUI never activates when `AI_AGENT=1` or `AGENT=1` is set — falls back to `--format json` with an stderr warning.
- An **`Ingestor` trait** in `loran-index` is designed-in from Phase 1 even though v1 ships only `MarkdownPagesIngestor`. Phase 3 adds `DescribeIngestor` for SFRS-`describe` self-documenting Steelbore CLIs.

## Forthcoming workspace layout (Spec §3.1)

```
loran/
├── Cargo.toml, README.md, NOTICE.md, CONTRIBUTING.md, LICENSE
├── AGENTS.md, CLAUDE.md, SKILL.md
├── crates/
│   ├── loran-cli/      clap binary, dispatcher, exit codes
│   ├── loran-core/     orchestration, resolution chains
│   ├── loran-index/    index builder + Ingestor trait
│   ├── loran-pages/    page parser (TOML frontmatter + body)
│   ├── loran-render/   Markdown → terminal renderer
│   ├── loran-tldr/     tldr tarball fetch + cache + lookup
│   ├── loran-tui/      ratatui app
│   └── loran-mcp/      MCP server (Phase 3, read-only)
├── pages/              bundled fallback pages
└── xtask/              build/release/index-validate helpers
```

Crate-prefix naming follows Rust workspace convention; metallurgical naming applies only at the project/release level (Loran is acknowledged in Spec §13 as a heritage-engineering acronym exception; release codenames follow Ingot → Billet → Bloom).

## Canonical dependency picks (Spec §3.3)

Stick to these unless there is a documented reason to deviate:

`clap` (derive), `serde` + `serde_json` + `toml`, `pulldown-cmark`, `ratatui` + `crossterm`, `jiff` (not `chrono`), `ureq` + `rustls`, `tar` + `flate2`, `minisign-verify`, `nucleo-matcher`, `postcard`, `rmcp` (Phase 3), `thiserror` (libs) + `anyhow` (binaries), `tracing` + `tracing-subscriber`.

## Phasing

| Phase | Codename | Scope |
|-------|----------|-------|
| 1 | Ingot | Text-mode binary: workspace + posture files + global flags + JSON envelope + `list`/`show`/`help`/`find`/`search`/`categories` + index from bundled `pages/`. `Ingestor` trait in place. |
| 2 | Billet | Signed tarball update + overlays + TUI + `loran new` + `validate`. **The 1.0 milestone.** |
| 3 | Bloom | Read-only MCP + `loran schema` for agent function-calling + `DescribeIngestor` + Bravais/Ferrite OS overlay catalogs. |

Phases are **strict-sequential** (per Plan §2.1). Don't start Phase 2 work before Phase 1 ships.

## Build / test / dev commands

**None yet** — there is no `Cargo.toml`. Once Phase 0 lands, the standard pre-commit gate per Plan §2.3 will be:

```sh
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

Until then, document changes are the only work product. Keep the `.md` files authoritative and let the `.odt` / `.md.pdf` siblings regenerate from them.

## Conventions for editing the planning docs

- SPDX header `<!-- SPDX-License-Identifier: GPL-3.0-or-later --> <!-- Copyright (c) 2026 Mohamed Hammad -->` at the top of every Markdown file.
- All dates and timestamps are ISO 8601 with `Z` suffix (UTC, no offsets).
- Task IDs in the TODO are append-only — never renumber, even if a task is deleted (leave the ID retired).
- The TODO is **mutable**: add tasks during implementation if they emerge; keep IDs sequential within a phase.
- When the TODO is revised (after Phase 1 ships → v0.2; after Phase 2 ships → v0.3), decompose the next phase's stubs into full task lists.
- If you change documented behaviour during implementation, update the spec or PRD to match — divergence is not acceptable per Plan §2.3.

## Don't

- Don't run `git init`, `cargo init`, or create posture files without explicit instruction — those are tracked tasks (`LOR-P000-001` etc.) the user will want to execute deliberately.
- Don't add `tokio` outside the `loran-mcp` crate.
- Don't apply the Steelbore palette to `loran help` capture output (intentional brand boundary in Spec §2 decision #11).
- Don't expose write-side verbs (`update`, `new`, `validate`) or `help` over the MCP surface.
- Don't fork tealdeer / tlrc — reuse patterns only.
- Don't use `apt`, `dnf`, `pacman`, or other system-distro package managers. Use Guix/Nix/Cargo/Homebrew/Flatpak/Snap (in that order) per `steelbore-missing-pkg`.
