+++
name = "gron"
category = "data-processing"
summary = "Make JSON greppable. Flattens nested structures into discoverable assignment lines."
replaces = []
safe_alias_for = []
pairs_with = ["jaq", "rg"]
official = "https://github.com/tomnomnom/gron"
tldr_page = "gron"
written_in = "go"
since = "bravais@0.1"
tags = ["json", "text"]
aliases = []
+++

## Steelbore notes

`gron` flattens JSON into one-line-per-key assignments so you can pipe it to `rg` and discover the exact path you need without learning a query language. Once you know the path, switch to `jaq` for the real extraction. Steelbore recommends `gron` for the discovery phase, `jaq` for the execution phase.

```sh
xh api.example.com/users | gron | rg 'email'
# json[0].email = "alice@example.com";
# json[1].email = "bob@example.com";
```

## Recommended usage

```sh
gron file.json | rg 'pattern'                 # find a path
gron --ungron path.json                       # round-trip back to JSON
gron --values file.json                       # values only
```

## Why it lives next to jaq

`gron`'s output is line-oriented, which means `rg` / `awk` / `cut` all work on it without ceremony. Use it when you do not yet know the schema; the moment you do, write a `jaq` query for repeatability.

## Pairs with

- **jaq** — discover with `gron`, extract with `jaq`.
- **rg** — the canonical grep for the flattened output.
