+++
name = "just"
category = "shell-utilities"
summary = "Modern command runner. `justfile` recipes with arguments, dependencies, and listing."
replaces = ["make"]
safe_alias_for = []
pairs_with = ["direnv", "starship"]
official = "https://just.systems"
tldr_page = "just"
written_in = "rust"
since = "bravais@0.1"
tags = ["build", "shell"]
aliases = []
+++

## Spacecraft Software notes

`just` is the Spacecraft Software replacement for `make` when the goal is a project-local task runner rather than a build system. Recipes are scripts, not declarative dependency graphs; arguments are first-class; tab-indentation rules don't bite.

`safe_alias_for` is empty because `make`'s rule semantics (file timestamps, implicit rules) are not what `just` offers; aliasing would mislead.

## Recommended setup

```just
# justfile at repo root
default:
    @just --list

test:
    cargo test --workspace

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

ci: clippy test
    cargo xtask check-spdx
```

Run with `just`, `just test`, `just ci`. `just --list` lists every recipe with its doc comment.

## Differences from `make`

- Recipes are shell scripts, not implicit-rule magic. Predictable.
- Recipe arguments: `release version:` becomes `just release v0.2.0`.
- `@command` silences echo for that line; no `.SILENT:` ritual.
- Cross-platform; uses `sh` on Unix and `cmd`/`pwsh` on Windows by default.

## Pairs with

- **direnv** — recipes inherit the active per-project environment automatically.
- **starship** — when a recipe is long-running, `cmd_duration` flags it on the prompt.
