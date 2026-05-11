+++
name = "procs"
category = "process-management"
summary = "Modern ps replacement with colour, search, and tree view."
replaces = ["ps"]
safe_alias_for = []
pairs_with = ["bottom"]
official = "https://github.com/dalance/procs"
tldr_page = "procs"
written_in = "rust"
since = "bravais@0.1"
tags = ["process", "monitoring"]
aliases = []
+++

## Steelbore notes

`procs` is the Steelbore-canonical replacement for `ps`. It surfaces the same kernel data with sane column defaults, colour, fuzzy filtering, and a tree-view mode that `ps` requires extension flags to approximate.

`safe_alias_for` is empty: `ps` has decades of stable BSD/SysV-style flag behaviour that scripts depend on. `procs` is the modern surface for interactive use; keep `ps` in scripts.

## Recommended invocation

```sh
procs                            # all processes
procs python                     # filter to processes whose name/cmdline matches
procs --tree                     # parent/child tree
procs --sortd cpu                # sort by CPU, descending
procs --watch                    # auto-refresh (like `top`)
```

## Differences from `ps`

- Default columns are tuned for humans, not POSIX compatibility.
- Built-in filtering by command name without `grep`-ing.
- Tree view (`--tree`) without `pstree`.
- Watch mode (`--watch`) without `watch`-wrapping.

## Pairs with

- **bottom** — when `procs --watch` isn't enough, `bottom` gives you the full interactive system view.
