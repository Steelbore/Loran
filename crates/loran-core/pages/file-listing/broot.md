+++
name = "broot"
category = "file-listing"
summary = "Interactive TUI for navigating large directory trees with live fuzzy filtering."
replaces = ["tree"]
safe_alias_for = []
pairs_with = ["eza", "fd"]
official = "https://dystroy.org/broot"
tldr_page = "broot"
written_in = "rust"
since = "bravais@0.1"
tags = ["filesystem", "tui-friendly"]
aliases = ["br"]
+++

## Spacecraft Software notes

`broot` is the Spacecraft Software go-to when a directory tree is too large to skim with `eza --tree`. It opens a TUI, surfaces matches as you type, and can `cd` the parent shell to the selected directory via the `br` shell function it installs on first run.

`safe_alias_for` is empty because `broot` is an interactive TUI and `tree` is a one-shot text emitter; the output formats are not interchangeable.

## Recommended setup

```nushell
broot --install   # installs the `br` shell function, run once
```

After installation, run `br` to launch broot in a way that can change the shell's working directory on exit.

## Differences from `tree`

- Interactive, not one-shot. Use `eza --tree` for piping into another tool.
- Live filter: type any substring and matches appear instantly.
- Built-in actions: rm, mv, cd, edit, even git status visibility.

## Pairs with

- **eza** — `eza --tree` for non-interactive listings; `broot` for navigation.
- **fd** — both honour `.gitignore` by default.
