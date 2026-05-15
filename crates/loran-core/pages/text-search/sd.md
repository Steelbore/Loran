+++
name = "sd"
category = "text-search"
summary = "Intuitive find-and-replace CLI. Literal patterns by default, regex with -r."
replaces = ["sed"]
safe_alias_for = []
pairs_with = ["rg", "fd"]
official = "https://github.com/chmln/sd"
tldr_page = "sd"
written_in = "rust"
since = "bravais@0.1"
tags = ["text"]
aliases = []
+++

## Spacecraft Software notes

`sd` is Spacecraft Software's preferred substitution tool for the 95 % case where you want literal find-and-replace without `sed`'s sublanguage. The argument order — `sd <find> <replace> <files...>` — reads left-to-right and never requires escaping a delimiter.

`safe_alias_for` is empty: `sd`'s flags and stream semantics differ from `sed` enough that scripts using `sed` patterns will not work with `sd`.

## Recommended usage

```sh
echo "hello world" | sd hello goodbye        # → "goodbye world"
sd 'foo' 'bar' file.txt                      # in-place edit
sd -p 'foo' 'bar' file.txt                   # preview only, no write
sd -r '\d+' 'NUMBER' file.txt                # regex mode
```

## Differences from `sed`

- Literal strings by default; pass `-r` (`--regex`) only when you want regex.
- In-place editing is the default for file arguments; previewing requires `-p`.
- No `s/.../.../g` ritual: the find and replace are separate positional args.

## Pairs with

- **rg** — `rg -l 'pattern' | xargs sd 'pattern' 'replacement'`
- **fd** — `fd -e txt -x sd 'old' 'new' {}`
