+++
name = "fd"
category = "file-search"
summary = "Simple, fast user-friendly alternative to find."
replaces = ["find"]
safe_alias_for = []
pairs_with = ["rg", "bat", "eza"]
official = "https://github.com/sharkdp/fd"
tldr_page = "fd"
written_in = "rust"
since = "bravais@0.1"
tags = ["filesystem", "search"]
aliases = []
+++

## Steelbore notes

`fd` is the Steelbore-canonical filesystem search tool. It is dramatically faster than `find` on real trees, honours `.gitignore` by default, and uses sensible regex-by-default matching instead of `find`'s glob-by-default behaviour.

`safe_alias_for` is empty: `fd`'s defaults (`.gitignore`-aware, no implicit type, regex-by-default) differ from `find` enough that `alias find=fd` will break common shell idioms. Use `fd` explicitly; reach for `find` when you need POSIX semantics.

## Recommended invocation

```sh
fd pattern                       # case-insensitive regex from .
fd -e rs                         # every Rust file
fd -t f -e md                    # only regular .md files, no directories
fd 'pattern' /usr/local          # search a specific root
fd -H pattern                    # include hidden files
fd -X cmd                        # execute on each match (xargs-style)
```

## Differences from `find`

- Regex by default; pattern lives first instead of after `-name`.
- Smart-case: lower-case input is case-insensitive; mixed case is case-sensitive.
- Coloured output, parallel execution.
- Honours `.gitignore`; override with `-I` or `-uu`.

## Pairs with

- **rg** — `fd -e py | xargs rg pattern` for content search on a filename filter.
- **bat** — `fd config -X bat` previews every matching file.
- **eza** — `fd -t d -X eza -l --tree` lists directory matches as a tree.
