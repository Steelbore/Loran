+++
name = "hexyl"
category = "file-viewing"
summary = "Coloured hex viewer for the terminal. Distinguishes ASCII, control, and printable bytes."
replaces = ["xxd", "hexdump"]
safe_alias_for = []
pairs_with = ["bat", "fd"]
official = "https://github.com/sharkdp/hexyl"
tldr_page = "hexyl"
written_in = "rust"
since = "bravais@0.1"
tags = ["binary", "tui-friendly"]
aliases = []
+++

## Spacecraft Software notes

`hexyl` is Spacecraft Software's preferred hex viewer. Output is colour-coded — null bytes, ASCII printable, ASCII whitespace, and high bytes each get their own hue — which makes scanning binary data far faster than monochrome `xxd`.

`safe_alias_for` is empty because the output format is not byte-compatible with `xxd` / `hexdump`; tools that parse that output (rare) would break.

## Recommended usage

```sh
hexyl --length 256 file.bin
hexyl --skip 0x100 file.bin           # start at byte offset
hexyl --color always file.bin | less -R
```

## Differences from `xxd` / `hexdump`

- Coloured output by default.
- ANSI escape support detects pipes/TTY and adjusts.
- `--length` / `--skip` accept hex, decimal, and `KiB/MiB` units.

## Pairs with

- **bat** — for the text counterpart; both share a similar visual language.
- **fd** — `fd -t f -e bin -x hexyl --length 64` to peek at every binary in a tree.
