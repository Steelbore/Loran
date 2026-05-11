<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# Contributing to Loran

Thanks for your interest in **Loran** — the Steelbore reference manual. This document is the human contributor on-ramp; `AGENTS.md`, `CLAUDE.md`, and `SKILL.md` cover the machine / agent side.

## Ground rules

- **License:** GPL-3.0-or-later. By submitting a contribution you agree your work is licensed under GPL-3.0-or-later and that you have the right to submit it under that license.
- **SPDX headers** at the top of every source file you create or substantially modify:
  ```rust
  // SPDX-License-Identifier: GPL-3.0-or-later
  // SPDX-FileCopyrightText: 2026 <Your Name>
  ```
- **DCO sign-off** (`git commit -s`) is required on every commit. This appends a `Signed-off-by:` trailer attesting to the [Developer Certificate of Origin](https://developercertificate.org/).
- **Commit messages:** short imperative-mood subject (≤72 chars), blank line, then a body explaining the *why*. Reference the affected TODO task ID (e.g. `LOR-P001-042`) when applicable.
- **Branching:** feature branches off `main`, squash-or-rebase merge.

## Scope

Loran is governed by three documents that define what's in scope:

- `loran-prd-v0_1.md` — the *what* (PRD)
- `loran-spec-v0_2.md` — the *how* (canonical technical reference)
- `loran-plan-v0_1.md` — work-package decomposition with dependency graph
- `loran-todo-v0_1.md` — task-level checklist with stable IDs

Contributions that fall outside these documents are unlikely to be accepted unless they accompany a documented spec or plan revision.

## Development setup

Requires Rust 1.88.0 or later (pinned via `rust-toolchain.toml`).

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
```

Run all four cleanly before opening a PR. CI mirrors the same gate.

## Pull-request checklist

Before opening a PR:

- [ ] All commits carry DCO sign-off.
- [ ] All new source files carry SPDX headers.
- [ ] `cargo fmt --check` is clean.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo audit` reports no advisories (or new advisories are documented).
- [ ] If a TODO task is cleared, `loran-todo-v0_1.md` is updated in the same PR.
- [ ] If spec or PRD behaviour changed, the affected document is updated in the same PR.
- [ ] New CLI features honour Steelbore SFRS v1.0.0 §3–§11 (global flags, JSON envelope, exit-code map, structured errors to stderr).

## Maintainer discretion

Per Steelbore Standard v1.1 §5.4: PR acceptance, feature scope, naming, architecture, and roadmap decisions are at the maintainer's sole discretion. Rejection reflects fit, not quality. The maintainer commits to giving honest, specific feedback on rejected PRs so contributors can recalibrate.

## Security reporting

Found a security issue? **Do not file a public issue.** Email the maintainer directly at <Mohamed.Hammad@Steelbore.com> with details and a reproduction. You will receive an acknowledgement within reasonable best-effort time. Coordinated disclosure timelines will be agreed case-by-case; there is no published embargo policy at this stage of the project.

## Code of conduct

Be kind. Assume good faith. The Steelbore project values rigour over speed and correctness over cleverness; feedback in that spirit is always welcome.

---

*Forged in Steelbore.*
