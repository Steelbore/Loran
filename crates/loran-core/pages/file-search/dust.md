+++
name = "dust"
category = "file-search"
summary = "Disk-usage analyser. Shows where space is going in a colour bar-chart, sorted by size."
replaces = ["du"]
safe_alias_for = []
pairs_with = ["fd", "eza"]
official = "https://github.com/bootandy/dust"
tldr_page = "dust"
written_in = "rust"
since = "bravais@0.1"
tags = ["filesystem", "tui-friendly"]
aliases = []
+++

## Spacecraft Software notes

`dust` answers "where did my disk go?" in a single screen: a tree of the largest directories sorted by size, with a relative bar chart to the right. It is the Spacecraft Software replacement for the classic `du | sort -h | tail` pipeline.

`safe_alias_for` is empty because `du`'s flag grammar and output format are different; scripts that parse `du -sh` would break with `dust`.

## Recommended usage

```sh
dust              # current directory
dust /var/log     # specific path
dust -d 2         # depth limit
dust -n 50        # top 50 entries
dust -r           # reverse sort (largest first; default already largest)
```

## Differences from `du`

- Default output is a tree with a bar chart, sized for an 80-column terminal.
- Honours `.gitignore` only if you pass `-i`.
- No `--max-depth=N | sort | head` ritual — `-d N` and `-n N` are built in.

## Pairs with

- **fd** — `fd -t d -d 1 . | xargs dust -d 0` for a one-per-line size report.
- **eza** — `eza --total-size` for a similar idea at the file-level columns.
