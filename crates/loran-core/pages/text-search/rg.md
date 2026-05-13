+++
name = "rg"
category = "text-search"
summary = "ripgrep — recursively search files for a regex pattern. The Steelbore default."
replaces = ["grep"]
safe_alias_for = []
pairs_with = ["fd", "bat"]
official = "https://github.com/BurntSushi/ripgrep"
tldr_page = "rg"
written_in = "rust"
since = "bravais@0.1"
tags = ["search", "regex"]
aliases = ["ripgrep"]
+++

## Steelbore notes

`rg` is the canonical Steelbore replacement for `grep`. It is dramatically faster than `grep -r` on real codebases, honours `.gitignore` by default, and has a more ergonomic default flag set.

`safe_alias_for` is empty: `rg`'s defaults (recursive, gitignore-aware, hidden-file-skipping) differ enough from `grep`'s defaults that `alias grep=rg` will silently change the meaning of shell scripts. Use `rg` explicitly; keep `grep` for `grep`-flavour invocations.

## Recommended invocation

```sh
rg pattern                       # recursive from .
rg -t rust pattern               # only Rust files
rg --files | rg test             # list files matching a name pattern
rg -F 'literal text'             # fixed-string mode
rg -uu pattern                   # also search ignored / hidden files
```

## Differences from `grep`

- Recursive by default.
- `.gitignore` / `.ignore` honoured by default — override with `-uu` / `-uuu`.
- File-type filters: `rg -t rust` / `rg -t py` instead of constructing `--include` masks.
- Unicode-aware regex; UTF-8 by default.

## Pairs with

- **fd** — `fd ext py -X rg pattern` searches Python files only.
- **bat** — pipe `rg` output through `bat -l log -p` for highlighted matches.
