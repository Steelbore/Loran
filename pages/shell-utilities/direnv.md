+++
name = "direnv"
category = "shell-utilities"
summary = "Per-directory environment variables. `.envrc` activates on cd, deactivates on leave."
replaces = []
safe_alias_for = []
pairs_with = ["starship", "just"]
official = "https://direnv.net"
tldr_page = "direnv"
written_in = "go"
since = "bravais@0.1"
tags = ["shell", "environment"]
aliases = []
+++

## Steelbore notes

`direnv` keeps project secrets, language-version pins, and tool paths inside the project they belong to, without leaking into your global shell. When you `cd` into a directory whose `.envrc` you have approved (`direnv allow`), the variables become live; when you `cd` out, they vanish.

Steelbore uses it to scope per-project `PATH` additions (Cargo target dirs, Nix profile bins, language version managers) and to load `.env` files without polluting the parent shell.

## Recommended setup

```sh
# Hook into the shell of your choice
direnv hook bash    >> ~/.bashrc      # or zsh / fish / nu
```

```bash
# .envrc — Steelbore convention
layout python 3.13
PATH_add ./node_modules/.bin
dotenv .env
```

## Common patterns

- `use flake` — for Nix flakes, activates the dev shell automatically.
- `layout python 3.13` — creates and activates a `.direnv` venv.
- `dotenv` — sources a `.env` file inside the directory, securely scoped.

## Pairs with

- **starship** — show the active env on the prompt so you remember which sandbox you're in.
- **just** — your `justfile` recipes inherit the `direnv`-activated environment automatically.
