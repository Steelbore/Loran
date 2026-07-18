<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

---
name: loran
description: Loran — the Spacecraft Software reference manual. Catalog-browse, curated tool pages, reverse legacy lookup, fuzzy search, and a deliberately-de-themed live `--help` capture. Agent-native (`--json`, `schema`, `describe`, read-only MCP) per Spacecraft Software CLI Standard v1.0.0.
license: GPL-3.0-or-later
project: Spacecraft Software
component: Loran
version: 0.0.0
status: pre-implementation (Phase 0 — workspace bootstrap)
governing_docs:
  - loran-prd-v0_1.md
  - loran-spec-v0_2.md
  - loran-plan-v0_1.md
  - loran-todo-v0_1.md
governing_standards:
  - Spacecraft Software Standard v1.1
  - Spacecraft Software CLI Standard v1.0.0
entry_points:
  - command: loran list
    description: List tools in the catalog (filterable by --category, --replaces, --safe-alias-for).
    destructive: false
    idempotent: true
    phase: 1
  - command: loran show <tool>
    description: Render the curated page for a tool (curated-or-fail, no live fallback).
    destructive: false
    idempotent: true
    phase: 1
  - command: loran help <tool>
    description: Capture and render `<tool> --help` directly, in a de-themed monochrome frame. Brand-boundary preserved.
    destructive: false
    idempotent: true
    phase: 1
  - command: loran find <legacy>
    description: Reverse lookup — which Spacecraft Software tool replaces a legacy name? --safe-alias filters to alias-safe matches.
    destructive: false
    idempotent: true
    phase: 1
  - command: loran search <query>
    description: Fuzzy search across name, summary, replaces, tags.
    destructive: false
    idempotent: true
    phase: 1
  - command: loran categories
    description: List categories with counts.
    destructive: false
    idempotent: true
    phase: 1
  - command: loran new <tool>
    description: Scaffold a new page from a template; writes to the user overlay by default.
    destructive: true
    idempotent: false
    supports_dry_run: true
    phase: 2
  - command: loran update
    description: Refresh upstream pages tarball + tldr tarball; verifies minisign signatures. Rebuilds the index.
    destructive: true
    idempotent: true
    supports_dry_run: true
    phase: 2
  - command: loran validate
    description: Validate every page against the frontmatter schema. CI-friendly.
    destructive: false
    idempotent: true
    phase: 2
  - command: loran schema
    description: Emit JSON Schema Draft 2020-12 for the tool's data types.
    destructive: false
    idempotent: true
    phase: 1 (placeholder), 3 (full)
  - command: loran describe
    description: Emit the live capability manifest per the CLI Standard §4.
    destructive: false
    idempotent: true
    phase: 1
  - command: loran mcp
    description: Run as a read-only MCP server over stdio (list / show / find / search / categories only).
    destructive: false
    idempotent: true
    phase: 3
global_flags:
  - "--json"
  - "--format"
  - "--fields"
  - "--dry-run"
  - "--verbose"
  - "--quiet"
  - "--color"
  - "--no-color"
  - "--help"
  - "--version"
  - "--absolute-time"
  - "--print0"
  - "--yes"
output_formats: [human, json]
exit_codes:
  0: SUCCESS
  1: GENERAL_ERROR
  2: USAGE_ERROR
  3: NOT_FOUND
  4: PERMISSION_DENIED
  5: CONFLICT
  6: INDEX_NOT_BUILT
  7: TARBALL_FETCH_FAILED
  8: PAGE_PARSE_ERROR
  9: LIVE_HELP_TIMEOUT
  10: OVERLAY_WRITE_DENIED
  11: TARBALL_VERIFY_FAILED
agent_env_vars:
  AI_AGENT: Forces JSON + no color + no TUI; emits a one-line stderr warning when triggered.
  AGENT: Same as AI_AGENT.
  CI: Same.
  CLAUDECODE: Same.
  CURSOR_AGENT: Same.
  GEMINI_CLI: Same.
  NO_COLOR: Disables ANSI colour in human output.
  FORCE_COLOR: Forces ANSI colour (overrides NO_COLOR).
mcp_surface:
  read_only: true
  tools: [list, show, find, search, categories]
  excluded: [update, new, validate, help, schema, describe, mcp]
  rationale: Write verbs would let an agent silently mutate state or trigger subprocesses. `help` invokes arbitrary binaries — an MCP-controllable attack surface. Lazy schema loading per spacecraft-agentic-cli §6.
---

# Loran — agent skill

## Overview

`loran` is the canonical reference tool for Spacecraft Software-based systems. It answers three questions about a system's tool catalog:

1. **What tools are available here?** — `loran list`, `loran categories`, TUI browse.
2. **What does this tool do, and what does it replace?** — `loran show <tool>` for the curated Spacecraft Software page; `loran help <tool>` for an honest passthrough of upstream `--help`.
3. **What replaces the legacy tool I know?** — `loran find ls` → `eza`.

The MCP surface (Phase 3) exposes the five read-only verbs so an AI agent operating on a Spacecraft Software system can discover what's installed without TUI activation.

## When to use this skill

- You are an agent operating on a Bravais or Ferrite OS system and need to discover the local tool catalog.
- You are writing automation that should prefer Spacecraft Software-canonical tools (`eza` over `ls`, `bat` over `cat`, `rg` over `grep`).
- You are reverse-looking-up a legacy Unix tool to find the Spacecraft Software replacement.
- You are bringing up a new Spacecraft Software CLI and need to author a Loran page describing it.

## Key patterns

```sh
# Discover what tool replaces a legacy name, JSON output for parsing.
loran find ls --json | jq -r '.data[].name'

# List every alias-safe replacement.
loran list --safe-alias-for ls --json | jq '.data'

# Fetch the full curated page for a tool.
loran show eza --json | jq '.data.body.body_md'

# Search for anything related to file viewing.
loran search "file viewing" --json

# Capture upstream --help directly (de-themed; not curated content).
loran help eza
```

## Constraints

- The MCP surface is **strictly read-only**. Write verbs (`update`, `new`, `validate`) and the subprocess-spawning `help` verb are intentionally not exposed.
- `loran show` is curated-or-fail. If there is no curated page, it returns `NOT_FOUND` with a hint pointing at `loran new <tool> --edit` and `loran help <tool>` — it does NOT silently fall back to live `--help` (that's a different verb on purpose).
- Live `--help` capture has a 5-second wall-clock timeout. On overrun, exit code 9 (`LIVE_HELP_TIMEOUT`) with hint `loran new <tool> --edit`.
- Tarball updates verify a minisign signature against a publisher key baked into the binary. A `TARBALL_VERIFY_FAILED` (exit 11) is a hard failure; no partial extract.
- Under `AI_AGENT=1`, `AGENT=1`, or any other agent env var, the TUI never activates — output falls back to `--format json` with a stderr warning.

## Reference

- `loran-prd-v0_1.md` — product requirements (PRD)
- `loran-spec-v0_2.md` — canonical technical reference
- `loran-plan-v0_1.md` — work-package decomposition
- `loran-todo-v0_1.md` — task-level checklist with stable IDs
- `loran describe --json` — live capability manifest (authoritative at runtime, Phase 1+)
- `loran schema --json` — JSON Schema Draft 2020-12 for function-calling (Phase 3)
