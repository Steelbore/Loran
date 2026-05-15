+++
name = "dasel"
category = "data-processing"
summary = "Multi-format query and edit — JSON, YAML, TOML, XML — one selector syntax."
replaces = ["yq"]
safe_alias_for = []
pairs_with = ["jaq", "miller"]
official = "https://daseldocs.tomwright.me"
tldr_page = "dasel"
written_in = "go"
since = "bravais@0.1"
tags = ["json", "yaml", "toml"]
aliases = []
+++

## Spacecraft Software notes

`dasel` is the Spacecraft Software default when the format is YAML or TOML rather than JSON. One selector grammar works across JSON / YAML / TOML / XML, so you stop context-switching between `jaq` (JSON-only) and `yq` (YAML, with several incompatible forks).

```sh
dasel -f Cargo.toml '.package.name'
dasel -f config.yaml '.servers.first().host'
dasel put -f config.yaml -t string '.debug' 'true'   # in-place edit
```

## Recommended usage

```sh
dasel -f data.json '.users.[0].name'
dasel -r yaml '.a.b.c' < file.yaml             # explicit format
dasel -f Cargo.toml '.dependencies.*.version'  # iterate
dasel convert -f Cargo.toml -t json            # cross-format conversion
```

## Differences from `jaq` / `yq`

- One binary across formats; consistent selectors regardless of input.
- In-place editing for YAML/TOML — `jaq` is JSON-only and stream-only.
- Slower than `jaq` on huge JSON; prefer `jaq` for that case.

## Pairs with

- **jaq** — when the input is JSON, `jaq` is usually faster and more expressive.
- **miller** — `dasel` for hierarchical formats, `miller` for tabular ones.
