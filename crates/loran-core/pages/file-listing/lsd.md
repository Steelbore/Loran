+++
name = "lsd"
category = "file-listing"
summary = "Drop-in `ls` replacement with colours and icons. Conservative alternative to eza."
replaces = ["ls"]
safe_alias_for = ["ls"]
pairs_with = ["bat", "fd"]
official = "https://github.com/lsd-rs/lsd"
tldr_page = "lsd"
written_in = "rust"
since = "bravais@0.1"
tags = ["filesystem", "tui-friendly"]
aliases = []
+++

## Spacecraft Software notes

`lsd` (LSDeluxe) is Spacecraft Software's recommended alternative when a user wants something closer to GNU `ls` than `eza`. Argument grammar is `ls`-compatible enough that `alias ls=lsd` rarely breaks scripts that inspect filenames or sizes; that is why `safe_alias_for` includes `ls`.

Pick `eza` when you want git integration and tree mode out of the box. Pick `lsd` when you want `ls`-compatible flags with colour + icons and nothing else.

## Recommended aliases

```nushell
alias ls = lsd
alias ll = lsd -l
alias la = lsd -la
```

## Differences from `ls`

- Colour and Nerd-Font icons by default.
- `--tree` mode for recursive listing.
- Stable column layout — pipes still work for typical `awk '{print $9}'` extraction.

## Pairs with

- **bat** — pipe a single file from `lsd -l` straight into `bat` for highlighted viewing.
