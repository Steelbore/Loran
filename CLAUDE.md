<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# CLAUDE.md

This file provides Claude-Code-specific guidance for working with the Loran repository. For coding conventions, forbidden patterns, the pre-commit gate, repository layout, and CLI conformance rules, **read [AGENTS.md](AGENTS.md) first** — those are tool-agnostic and apply to every coding agent. This file only adds what's specific to Claude Code (skills system, planning-doc reading order, locked design decisions worth knowing before you start).

## Repository status

Phase 0 (workspace bootstrap) is in progress. Until Phase 1 lands, there are no sub-commands to run, no tests beyond placeholders, and most of the architecture described below exists only in the planning docs.

## The four governing documents (read in this order)

1. **`loran-prd-v0_1.md`** — *what* Loran must accomplish. Goals (G-01…G-10), quality goals (Q-01…Q-07), non-goals, user stories, functional + non-functional requirements.
2. **`loran-spec-v0_2.md`** — *how* Loran will be built. Canonical technical reference: workspace layout, dependency stack, resolution chains for `show`/`help`, page frontmatter schema, JSON envelope, exit codes, filesystem layout, tarball update mechanism. **Spec v0.2 supersedes the older `lodestone-spec-v0.1.md`** — that file describes the project under its previous name (Lodestone) and is kept only as historical context.
3. **`loran-plan-v0_1.md`** — work packages with sizing, dependencies, and acceptance criteria. Operationalises the PRD against the spec.
4. **`loran-todo-v0_1.md`** — task-level decomposition with stable IDs (`LOR-PXXX-NNN`). Phase 0 and Phase 1 are detailed; Phase 2/3 are stubbed until earlier phases ship.

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
- Tarball update model (no runtime git dependency), **minisign + ed25519** signatures verified against a publisher key baked into the binary via `include_bytes!`. Key rotation requires a new Loran release.
- **No `tokio` on the v1 fast path.** Tarball fetch is one synchronous `ureq` request. Async is confined to the Phase 3 `loran-mcp` crate.
- **MCP surface is read-only** (`list`, `show`, `find`, `search`, `categories`). `update`, `new`, `validate`, and `help` are deliberately not exposed to agents.
- TUI never activates when `AI_AGENT=1` or `AGENT=1` is set.
- An **`Ingestor` trait** in `loran-index` is designed-in from Phase 1 even though v1 ships only `MarkdownPagesIngestor`.

## Don't

- Don't run `git init`, `cargo init`, or create posture files in a session that wasn't explicitly authorized for that work — those are tracked tasks (`LOR-P000-001` etc.).
- Don't add `tokio` outside the `loran-mcp` crate.
- Don't apply the Steelbore palette to `loran help` capture output (Spec §2 decision #11).
- Don't expose write-side verbs (`update`, `new`, `validate`) or `help` over the MCP surface.
- Don't fork tealdeer / tlrc — reuse patterns only.
- Don't use `apt`, `dnf`, `pacman`, or other system-distro package managers. Use Guix/Nix/Cargo/Homebrew/Flatpak/Snap per `steelbore-missing-pkg`.
- Don't bypass the DCO sign-off requirement or skip the pre-commit gate (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`).
