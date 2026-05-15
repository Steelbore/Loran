<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# Loran — Compliance Audit Log

This file records the Standard §14 + Spec §13 + PRD §19 compliance audits performed at each Phase Definition-of-Done boundary. Every audit appends; nothing is rewritten.

---

## Audit 2026-05-12 — Phase 1 (Ingot) Definition of Done

**Auditor**: Maintainer self-audit at WP-P1.19 completion.
**Commit at audit time**: branch `main`, post-Sub-phase 1E.
**Catalog**: 8 seed pages spanning 7 categories (Spec M-01 target is 25+; partial — Phase 1B `WP-P1.03` content tail deferred).

### Spacecraft Software Standard v1.1 §14 + Spec §13 row-by-row

| § | Requirement | Status | Evidence |
|---|---|---|---|
| 2 | Metallurgical naming | ✓ (with documented exception) | Loran is a heritage engineering acronym (LORAN — radio navigation). Codename exception granted in Spec §13.1. Release codenames follow Ingot → Billet → Bloom. |
| 3.1 | Memory safety | ✓ | 100 % safe Rust. `#![forbid(unsafe_code)]` on `loran-core`, `loran-pages`, `loran-render`. `#![deny(unsafe_code)]` on `loran-index`, `loran-tldr`, `loran-tui`, `loran-mcp`. Workspace lint denies `unsafe_code` globally. `cargo clippy --workspace --all-targets -- -D warnings` clean. |
| 3.2 | Performance / concurrency | ✓ | Phase 1 fast path fully synchronous — no `tokio`. NFR-001 (`loran show` ≤ 50 ms) and NFR-002 (`loran list` ≤ 100 ms over a 1k-page catalog) verified by `crates/loran-core/tests/nfr.rs` (release-mode). Async confined to the Phase 3 `loran-mcp` crate. |
| 3.3 | Hardened security; PQC | ✓ (Phase 2 surface) | `rustls` not yet wired (tarball fetch is Phase 2). Live `--help` capture is PATH-resolved (never accepts a path-shaped name), spawns with no shell, applies a 5 s wall-clock timeout, kills on overrun. `cargo audit --deny warnings` clean on the published dep graph. |
| 4 | GPL-3.0-or-later + SPDX | ✓ | LICENSE verbatim. SPDX headers on every `.rs`, `.toml`, `.sh`, `.nix` file in the workspace. Enforced by `cargo xtask check-spdx` (run in CI). Markdown / TOML data files (`pages/`, `pages/categories.toml`) carry SPDX comments where the format allows. |
| 5.1 | Default personal-hobby posture | ✓ | Stated in `README.md` Project Posture section. No service-level commitments; warranty / liability waived in `NOTICE.md`. |
| 5.2 | Required posture files | ✓ | `README.md`, `NOTICE.md`, `CONTRIBUTING.md`, `LICENSE` all present at repo root from the first commit. |
| 6.1 | POSIX-compliant CLI | ✓ | Default text output is tab-separated, ANSI-free, and parseable with `cut`/`awk`/`grep`/`sed` alone. `--print0` flag wired (Phase 1B). No locale-dependent output. |
| 7 | PFA: no tracking, minimal perms, local | ✓ | Zero telemetry. Filesystem reads only (within `$XDG_DATA_HOME`/`$XDG_CACHE_HOME`/`$XDG_CONFIG_HOME` semantics — Phase 2 enforces). Outbound HTTPS reserved for `loran update` (Phase 2). All data local. |
| 8 | CUA + Vim bindings | N/A (Phase 1) | TUI deferred to Phase 2. Text-mode CLI has no key-bindings to honour either scheme. |
| 9 | Spacecraft Software palette; Void Navy bg | N/A (Phase 1) | TUI deferred to Phase 2. Phase 1 deliberately emits no ANSI escapes (PRD NFR-050). `loran help` capture frame is deliberately de-themed (monochrome ASCII) per Spec §2 decision #11 — brand boundary holds. |
| 10 | Material Design / WCAG 2.1 AA | N/A | No GUI surface in v1. |
| 11 | FOSS fonts | N/A | Terminal app, uses user's terminal font. |
| 12 | ISO 8601 UTC, Z suffix, 24h, metric | ✓ | All timestamps in the JSON envelope, `live_help` captures, and stored data carry the `Z` suffix. Asserted by `debug_assert!` in `loran_cli::envelope::serialize_timestamp` and verified in every integration test that inspects a timestamp. `jiff` is the only time crate (no `chrono`, no `time`). |
| 13 | Attribution (maintainer, URL, copyright) | ✓ | `--version` (human) prints `Maintained by Mohamed Hammad <…@…> / Project: https://Loran.SpacecraftSoftware.org/ / Source: https://github.com/Spacecraft-Software/Loran`. `--version --json` carries the same as `metadata.maintainer` / `metadata.website` / `metadata.source`. `--help` long footer carries the same. README "Maintainer" section + copyright year present. |

### Spec §13 compliance audit

All rows match Standard §14 above with no Spec-specific deviations.

### PRD §19 compliance audit

| PRD goal | Status | Evidence |
|---|---|---|
| G-01 catalog browse without prior knowledge | ✓ | `loran list`, `loran categories` |
| G-02 reverse-lookup legacy names | ✓ | `loran find <legacy>` |
| G-03 `safe_alias_for` visibility | ✓ | Frontmatter parsed (`loran-pages`), surfaced in `loran show --json` `data.safe_alias_for` + `loran find --safe-alias` |
| G-04 `pairs_with` visibility | ✓ | Frontmatter parsed, surfaced in `loran show --json` `data.pairs_with` |
| G-05 agent discovery via `--json` / MCP | ✓ (`--json`) / Phase 3 (MCP) | Every sub-command implements `--json`. MCP server lands in Phase 3. |
| G-06 catalog refresh via signed source | Phase 2 | `loran update` is a stub today; minisign-verified tarball fetch lands in WP-P2.07–WP-P2.10. |
| G-07 overlay layers | Phase 2 | `BundledPagesIngestor` is the only ingester live in Phase 1. Overlay machinery + `MarkdownPagesIngestor`-on-overlays land in WP-P2.13. |
| G-08 page authoring workflow | Phase 2 | `loran new` is a stub today; scaffolding lands in WP-P2.14/15. |
| G-09 inter-CLI integration (`loran show <self>`) | Future | No other Spacecraft Software CLI references `loran show <self>` in its `--help` yet — adoption is downstream. |
| G-10 JSON Schema for agent function-calling | Partial (Page only) | `loran schema --json` emits a Draft 2020-12 schema for `Page` with `meta.placeholder = true`. Full schema (every command's data shape + exit codes + envelope) lands in Phase 3 WP-P3.01. |
| Q-01 sub-50 ms cold `show` | ✓ | NFR test asserts; release-mode passes well under the limit. |
| Q-02 100 % Rust, no `unsafe` outside FFI | ✓ | Zero `unsafe` blocks in the entire workspace; all crates forbid or deny `unsafe_code`. |
| Q-03 POSIX-compliant default output | ✓ | Verified by `cut`/`awk`-friendly tab-separated text output. Renderer test asserts zero `\x1b` bytes in output. |
| Q-04 WCAG 2.1 AA contrast | N/A (Phase 1) | TUI deferred to Phase 2. |
| Q-05 zero telemetry | ✓ | No analytics, no metrics, no network outside `loran update` (stubbed in Phase 1). |
| Q-06 atomic tarball updates | Phase 2 | `loran update` lands in WP-P2.10. |
| Q-07 actionable hint on every error | ✓ | `ExitCode::hint(&ErrorContext)` covers all 12 variants. Every CLI error path interpolates context (`tool`, `query`) into the hint. Unit-tested in `loran_cli::exit::tests`. |

### Test surface

| Suite | Tests | Notes |
|---|---|---|
| `loran-pages` | 22 unit + 1 doctest | Full frontmatter schema + every `PageError` variant |
| `loran-index` | 10 unit + 5 integration | Ingestor trait, MarkdownPagesIngestor, Index + secondary indexes |
| `loran-render` | 8 unit | ANSI-free output invariant + per-event coverage |
| `loran-core` | 38 unit + 5 integration + 2 NFR | Resolution chains + help capture + 1k-page NFR-002 |
| `loran-cli` | 7 unit (logging/color/envelope/version) + ~40 integration | Every sub-command happy path + JSON envelope shape + exit-code coverage + snapshots |
| **Total** | **~150** | All green locally and on CI |

### Benchmarks

`crates/loran-core/benches/resolution.rs` exercises `resolve_show` (hit + miss), `resolve_find` (broad + safe-alias), `resolve_search` (single-word + tag-match against both the bundled seed and a synthetic 1k-page catalog), and `Index::build`. Run with `cargo bench`. Regression detection is left as a Phase 2 cross-cutting concern — Phase 1 ships baseline numbers only.

### Documented deviations from the PRD M-targets

| M-target | Required | Actual | Disposition |
|---|---|---|---|
| M-01 | ≥ 25 curated pages | 8 | Spec §13 deviation. Build.rs + ingester ready; remaining work is pure markdown authoring under `pages/`. Tracked as a content-only Phase 1B tail. |
| M-02 | ≥ 20 legacy tools represented in `replaces` | 8 | Same. Will rise mechanically as content authoring completes. |
| M-03 | ≥ 1.5 average `pairs_with` per page | 0.875 (7 of 8 pages) | Same. |

### Sign-off

Phase 1 (Ingot) Definition of Done **passes** for every code-track requirement. The three M-target content deviations are documented above and are bounded to a known follow-up. Ready to tag `v0.1.0-ingot`.

— *Maintainer self-audit, 2026-05-12.*
