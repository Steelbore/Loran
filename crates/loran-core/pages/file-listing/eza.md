+++
name = "eza"
category = "file-listing"
summary = "Modern ls replacement. Spacecraft Software default for file listing."
replaces = ["ls"]
safe_alias_for = []
pairs_with = ["bat", "fd"]
official = "https://eza.rocks"
tldr_page = "eza"
written_in = "rust"
since = "bravais@0.1"
tags = ["filesystem", "tui-friendly"]
aliases = []
+++

## Spacecraft Software notes

`eza` is the Spacecraft Software-canonical file lister and is the modern successor to the unmaintained `exa`. It is the default surface every other Spacecraft Software tool assumes when it documents "your file listing".

## Recommended aliases

```nushell
alias ls = eza --git --icons
alias ll = eza -l --git --icons
alias tree = eza --tree
```

`safe_alias_for` is empty because `eza`'s default columns and flags differ enough from `ls` that scripts which inspect `ls` output by position can break. Use the aliases above for interactive work and call `eza` (or stock `ls`) explicitly from scripts.

## Differences from `ls`

- Tree mode is built in (`-T` / `--tree`), no separate `tree` binary required.
- Git status integration via `--git`.
- Icons for filetypes via `--icons` (Nerd Font required).
- Configurable column set via `--long`; sensible defaults out of the box.

## Pairs with

- **bat** — when you list a file with `eza`, pipe to `bat` for highlighting.
- **fd** — `fd <name> -X eza -l` lists everything that matched.
