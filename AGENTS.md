<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# AGENTS.md — Loran agent context (Codex, Cursor, Aider, generic)

Complements `CLAUDE.md`. Where `CLAUDE.md` is Claude-Code-specific (slash commands, skills system, planning-doc reading order), this file is the neutral agent-onboarding document consumed by OpenAI Codex CLI, Cursor, Aider, Continue, Gemini CLI, and similar tools.

## Project identity

- **Name:** Loran (LOng RAnge Navigation — heritage engineering acronym; reference grid for a Steelbore system)
- **Type:** Rust CLI + TUI reference manual for Steelbore-based systems
- **Status:** Pre-implementation (Phase 0 — workspace bootstrap)
- **Organisation:** Steelbore (umbrella project)
- **License:** GPL-3.0-or-later on every source file
- **Governing documents:** `loran-prd-v0_1.md` (what), `loran-spec-v0_2.md` (how), `loran-plan-v0_1.md` (work packages), `loran-todo-v0_1.md` (tasks)
- **Governing standards:** Steelbore Standard v1.1, Steelbore SFRS v1.0.0

## Coding conventions

- **Rust edition 2024**, stable toolchain pinned via `rust-toolchain.toml`.
- **Microsoft Pragmatic Rust Guidelines** apply (see `rust-guidelines` skill in Claude Code, or the equivalent house style).
- **SPDX headers** at the top of every `.rs` file and every `Cargo.toml`:
  ```rust
  // SPDX-License-Identifier: GPL-3.0-or-later
  // SPDX-FileCopyrightText: 2026 <Author>
  ```
- **`#![forbid(unsafe_code)]`** on `loran-core`, `loran-pages`, `loran-render` (pure Rust, no FFI).
- **`#![deny(unsafe_code)]`** on `loran-index`, `loran-tldr`, `loran-tui`, `loran-mcp` (may need narrow FFI escape hatches; any `unsafe` requires a `// SAFETY:` block).
- **Error handling:** `thiserror` in libraries (typed errors); `anyhow` only in binaries.
- **Time:** `jiff` everywhere (not `chrono`, not `time`). All timestamps ISO 8601 UTC with `Z` suffix; no local-time, no offsets, ever.
- **Async:** **no `tokio` outside `loran-mcp`.** Phase 1+2 are fully synchronous; tarball fetch is a one-shot `ureq` request.
- **Conventional commits** for subject lines (`feat:`, `fix:`, `chore:`, `docs:`, `build:`, `ci:`, `test:`, `refactor:`). Reference the affected TODO task ID (e.g. `LOR-P001-042`) in the body.
- **DCO sign-off** (`git commit -s`) required on every commit.

## Pre-commit gate (run all four cleanly before pushing)

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

CI mirrors the same gate plus an SPDX-header check.

## Forbidden patterns

- **`unsafe` blocks** without a structured `// SAFETY:` comment (Preconditions / Postconditions / Invariants).
- **`unwrap()` / `expect()`** in library code outside of test modules. Use `?` and let errors propagate.
- **`println!` / `eprintln!`** in library crates. Use `tracing` macros; routing to stdout/stderr is the CLI binary's responsibility.
- **Shell-specific idioms** in CLI output. Default text output must be POSIX-parseable with `grep`/`awk`/`cut`/`sed`.
- **External crates with non-GPL-3.0-compatible licenses.** `cargo deny check` enforces this in CI once configured.
- **Local-time / locale-dependent formatting.** ISO 8601 UTC + `Z` suffix, always.
- **Mocking the database / index in integration tests** — use real fixture trees under `tests/fixtures/`.
- **`tokio` in any crate except `loran-mcp`** (Phase 3 only).
- **The Steelbore palette** anywhere in `loran help` capture output — that frame is deliberately monochrome.

## CLI conformance (Phase 1+)

Every Loran sub-command must conform to **Steelbore SFRS v1.0.0**:

- **Global flags:** `--json`, `--format`, `--fields`, `--dry-run`, `--verbose`, `--quiet`, `--no-color`, `--color`, `--help`, `--version`, `--absolute-time`, `--print0`, `--yes`.
- **JSON envelope** per Spec §8: `{ metadata: { tool, version, command, timestamp, maintainer, website }, data: {...} }`.
- **Exit codes** 0–5 canonical (Success / GeneralError / UsageError / NotFound / PermissionDenied / Conflict) plus 6–11 Loran-specific (see Spec §9).
- **Structured errors** to stderr in JSON mode: `error: { code, exit_code, message, hint, timestamp, command, docs_url }`. Every error must carry an actionable `hint` per SFRS tips-thinking discipline.
- **Agent env-var detection:** if `AI_AGENT=1`, `AGENT=1`, `CI=true`, `CLAUDECODE=1`, `CURSOR_AGENT=1`, or `GEMINI_CLI=1` is set, the TUI must not activate. Fall back to `--format json` with a one-line stderr warning.
- **`NO_COLOR`** honoured everywhere; `FORCE_COLOR` / `CLICOLOR_FORCE` override only when explicit.

`loran describe --json` (Phase 1) returns the live capability manifest — consult it instead of hard-coding flag lists.

## Repository layout

```
loran/
├── Cargo.toml                       # workspace root
├── rust-toolchain.toml              # pin stable Rust
├── rustfmt.toml, .clippy.toml       # style config
├── LICENSE                          # GPL-3.0-or-later verbatim
├── README.md, NOTICE.md, CONTRIBUTING.md   # posture files (Standard §5.2)
├── AGENTS.md, CLAUDE.md, SKILL.md   # agent context (this file etc.)
├── loran-{prd,spec,plan,todo}-v0_X.md  # governing planning docs
├── crates/
│   ├── loran-cli/                   # clap binary, dispatcher, exit codes
│   ├── loran-core/                  # orchestration, resolution chains
│   ├── loran-index/                 # index builder + Ingestor trait
│   ├── loran-pages/                 # page parser (TOML frontmatter + body)
│   ├── loran-render/                # Markdown → terminal renderer
│   ├── loran-tldr/                  # tldr tarball fetch + cache + lookup
│   ├── loran-tui/                   # ratatui app (browse, detail, search)
│   └── loran-mcp/                   # MCP server (Phase 3, read-only)
├── pages/                           # bundled fallback pages (built into binary)
├── xtask/                           # build/release/index-validate helpers
└── .github/workflows/               # CI
```

## Phase boundary (where you can edit)

| Phase | Codename | Status |
|-------|----------|--------|
| 0 | (setup) | In progress |
| 1 | Ingot | Not started — text-mode binary, bundled catalog |
| 2 | Billet | Deferred — TUI, signed tarballs, overlays |
| 3 | Bloom | Deferred — MCP, schema, DescribeIngestor |

Don't start Phase 2 work before Phase 1 ships. Don't add tokio outside `loran-mcp`. Don't expose write verbs over MCP.

## Useful entry points

- `loran-todo-v0_1.md` — task-level checklist with stable IDs (`LOR-PXXX-NNN`)
- `loran describe --json` (Phase 1+) — live capability manifest
- `loran schema` (Phase 3) — JSON Schema Draft 2020-12 for function-calling
- `cargo xtask check-spdx` — SPDX-header lint across the workspace
