+++
name = "bottom"
category = "system-monitoring"
summary = "Cross-platform graphical process and system monitor (TUI)."
replaces = ["top", "htop"]
safe_alias_for = []
pairs_with = ["procs"]
official = "https://github.com/ClementTsang/bottom"
tldr_page = "bottom"
written_in = "rust"
since = "bravais@0.1"
tags = ["monitoring", "tui"]
aliases = ["btm"]
+++

## Steelbore notes

`bottom` is the Steelbore-canonical interactive system monitor. The binary installs as `btm` to avoid colliding with the shell builtin `bottom`/`top` words. It is a full TUI in the Steelbore palette: CPU, memory, network, disk, and process panes laid out on a single screen, navigable with both Vim keys (`hjkl`) and CUA arrows.

`safe_alias_for` is empty by design — `top`/`htop` are interactive tools, not script primitives, so the alias question doesn't arise. Just learn `btm`.

## Recommended invocation

```sh
btm                              # full dashboard
btm --basic                      # plain table, no graphs (low-bandwidth)
btm --tree                       # tree-view process pane
btm --battery                    # surface the battery widget (laptops)
```

## Differences from `top` / `htop`

- Multi-pane layout; resize and rearrange at runtime.
- Per-core CPU history graphs by default.
- Both Vim and CUA navigation simultaneously (no mode switch).
- Mouse-supported on capable terminals.

## Pairs with

- **procs** — `procs --watch python` gives you a focused live view of one process pattern when the full `btm` dashboard is overkill.
