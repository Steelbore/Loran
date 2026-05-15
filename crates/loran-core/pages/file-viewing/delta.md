+++
name = "delta"
category = "file-viewing"
summary = "Syntax-highlighting pager for git, diff, and grep output. Side-by-side and unified."
replaces = []
safe_alias_for = []
pairs_with = ["bat", "git-cliff"]
official = "https://dandavison.github.io/delta/"
tldr_page = "delta"
written_in = "rust"
since = "bravais@0.1"
tags = ["git", "diff", "tui-friendly"]
aliases = ["git-delta"]
+++

## Spacecraft Software notes

`delta` is the Spacecraft Software-canonical viewer for any diff-shaped output — `git diff`, `git log -p`, `git show`, and even `diff --color`. It replaces nothing per se because the underlying `git` and `diff` binaries still produce the data; `delta` just renders it.

Wire it into git globally:

```ini
# ~/.gitconfig
[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only

[delta]
    navigate = true
    line-numbers = true
    side-by-side = true
```

## Recommended setup

After the gitconfig snippet above, every `git diff`, `git log -p`, `git show`, and `git stash show -p` renders through `delta` automatically.

## Differences from plain `git diff`

- Syntax highlighting via `bat`'s themes.
- Side-by-side mode (`--side-by-side`) for wide terminals.
- `n` / `N` keys jump between files within the pager (`navigate = true`).
- Optional line numbers, full file rename detection, and merge conflict rendering.

## Pairs with

- **bat** — same syntax-highlighting engine; theme stays consistent across `bat` and `delta`.
- **git-cliff** — the changelog generator; preview its diffs with `delta`.
