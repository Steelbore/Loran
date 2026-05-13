+++
name = "lazygit"
category = "version-control"
summary = "Git TUI with stage-hunk-by-hunk, interactive rebase, and one-key commands."
replaces = []
safe_alias_for = []
pairs_with = ["delta", "git-cliff"]
official = "https://github.com/jesseduffield/lazygit"
tldr_page = "lazygit"
written_in = "go"
since = "bravais@0.1"
tags = ["git", "tui-friendly"]
aliases = ["lg"]
+++

## Steelbore notes

`lazygit` is the Steelbore TUI of record for git. It does not replace the `git` binary — every action shells out to real `git` — but it eliminates the muscle-memory tax of remembering plumbing flags for staging hunks, reordering commits during a rebase, and resolving merge conflicts.

The keymap is discoverable: `?` opens context-sensitive help in every panel.

## Recommended setup

```sh
lazygit              # opens in $PWD
lg                   # if you set the suggested alias
```

Wire it to use `delta` for diff rendering by leaving `core.pager = delta` in `~/.gitconfig`; `lazygit` honours it inside its diff panel.

## Why "lazy"

The name is tongue-in-cheek; the program is the opposite of lazy. The point is that it makes complex git operations one keystroke away — `s` stages a hunk, `c` commits, `P` pushes, `r` rebases.

## Pairs with

- **delta** — `lazygit`'s diff panel respects the gitconfig `core.pager = delta` setting.
- **git-cliff** — generate a changelog from the commits `lazygit` just helped you craft.
