+++
name = "bat"
category = "file-viewing"
summary = "cat with syntax highlighting, line numbers, and Git integration."
replaces = ["cat"]
safe_alias_for = ["cat"]
pairs_with = ["eza", "rg", "fd"]
official = "https://github.com/sharkdp/bat"
tldr_page = "bat"
written_in = "rust"
since = "bravais@0.1"
tags = ["file-viewing", "syntax-highlighting"]
aliases = []
+++

## Spacecraft Software notes

`bat` is the Spacecraft Software-canonical file viewer. Unlike `eza` vs `ls`, `bat` is alias-safe for `cat` — `bat`'s default detects piped output and gracefully falls back to plain text behaviour, so `alias cat=bat` does not break scripts.

## Recommended aliases

```nushell
alias cat = bat --paging=never
```

`--paging=never` is the Spacecraft Software default for the `cat`-alias surface: keep `bat` for interactive viewing (`bat file.rs` paginates) but never paginate inside a pipeline.

## Pager use

Loran's `loran help <tool>` capture engine uses `bat -pp` as the default pager when `$PAGER` and `$MANPAGER` are unset. `-pp` disables paging and decoration so output streams naturally.

## Differences from `cat`

- Syntax highlighting for ~200 languages out of the box.
- Line numbers (suppress with `-p`).
- Git integration: changed lines marked in the gutter.
- Built-in paging through `less` (override via `--paging` or `$BAT_PAGER`).

## Pairs with

- **rg** — `rg --files-with-matches pattern | xargs bat` shows matches with context.
- **fd** — `fd ext rs -X bat -p` views every Rust file plain.
