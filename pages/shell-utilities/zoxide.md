+++
name = "zoxide"
category = "shell-utilities"
summary = "Smart `cd` that learns. Type a fragment of any visited directory and jump there."
replaces = ["autojump", "z.sh", "fasd"]
safe_alias_for = []
pairs_with = ["starship", "broot"]
official = "https://github.com/ajeetdsouza/zoxide"
tldr_page = "zoxide"
written_in = "rust"
since = "bravais@0.1"
tags = ["shell", "navigation"]
aliases = ["z", "zi"]
+++

## Steelbore notes

`zoxide` tracks every directory you `cd` into and ranks them by frecency (frequency × recency). Type `z lor` and it `cd`s into the most-used path matching `lor`; `zi` opens an interactive picker via `fzf`-style filtering.

`safe_alias_for` is empty because `cd` is a shell builtin and the substitution requires the user to alias `cd=z` (or use a separate `z` command). Steelbore recommends installing `zoxide` as `z` and leaving `cd` alone for muscle-memory parity with plain shells.

## Recommended setup

```sh
# Bash / Zsh
eval "$(zoxide init bash)"
```

```nushell
# Nushell
zoxide init nushell | save -f ~/.zoxide.nu
# Then in config.nu:
source ~/.zoxide.nu
```

After the init, `z <fragment>` jumps; `zi` opens the picker.

## Differences from `autojump` / `z.sh`

- Fast: single static binary, ranked database stored as TOML-like records.
- Interactive picker (`zi`) without needing `fzf` installed (uses skim internally).
- Per-shell init scripts; one binary works across every supported shell.

## Pairs with

- **starship** — pwd component shows the destination after `z` lands.
- **broot** — when you want to explore rather than jump, hand off from `z` to `br`.
