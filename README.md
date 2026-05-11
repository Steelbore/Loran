<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# Loran

[![CI](https://github.com/Steelbore/loran/actions/workflows/ci.yml/badge.svg)](https://github.com/Steelbore/loran/actions/workflows/ci.yml)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](rust-toolchain.toml)

> The Steelbore reference manual.

**Loran** is the canonical, agent-friendly reference tool for Steelbore-based systems (Bravais, Ferrite OS, future distros). It is to Steelbore what `man` is to Unix and `info` is to GNU — a system-level handbook for every tool the system ships and recommends — with one critical difference: it is agent-native (`--json`, `schema`, MCP) from day one.

The name is a heritage engineering acronym: **LO**ng **RA**nge **N**avigation, the radio navigation infrastructure used by ships and aircraft from the 1940s until GPS retired it in 2010. Loran the tool is the precision reference grid for a Steelbore system.

## Overview

Loran answers three questions about the tool catalog of a Steelbore system:

1. **What tools are available here?** — categorised browse of the curated catalog.
2. **What does this tool do, and what does it replace?** — Steelbore-curated intro, with tldr fallback.
3. **What replaces the legacy tool I know?** — reverse lookup (`loran find ls` → `eza`).

A separate verb (`loran help <tool>`) captures live `--help` output from any binary on `$PATH`, rendered in a deliberately de-themed frame so curated content stays visually distinct from uncurated passthroughs.

## Project status

**Not yet released.** Loran is in Phase 0 (workspace bootstrap). The implementation roadmap ships in three phases:

| Phase | Codename | Scope |
|-------|----------|-------|
| 1 | Ingot | Text-mode binary; `list`/`show`/`help`/`find`/`search`/`categories`; bundled catalog. |
| 2 | Billet | TUI; signed tarball updates; overlays; page authoring. The 1.0 milestone. |
| 3 | Bloom | Read-only MCP surface; JSON Schema; auto-ingestion from SFRS `describe`. |

See `loran-prd-v0_1.md`, `loran-spec-v0_2.md`, `loran-plan-v0_1.md`, and `loran-todo-v0_1.md` for full requirements, design, plan, and task decomposition.

## Installation

Not yet packaged. Build from source once Phase 1 ships.

## Quickstart

Not yet available. Phase 1 will provide `loran list`, `loran show`, `loran find`, `loran search`, `loran help`, and `loran categories`.

## Project Posture

Loran is a **personal hobby project** under the Steelbore umbrella. Per Steelbore Standard v1.1 §5.1:

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

## Maintainer

**Mohamed Hammad** &lt;Mohamed.Hammad@Steelbore.com&gt;
Project URL: <https://Loran.Steelbore.com/>
Copyright © 2026 Mohamed Hammad.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for the verbatim text.

---

*Forged in Steelbore.*
