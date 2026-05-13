+++
name = "jaq"
category = "data-processing"
summary = "Faster, drop-in jq replacement with stricter semantics."
replaces = ["jq"]
safe_alias_for = ["jq"]
pairs_with = ["xh", "rg"]
official = "https://github.com/01mf02/jaq"
tldr_page = "jaq"
written_in = "rust"
since = "bravais@0.1"
tags = ["json", "filter"]
aliases = []
+++

## Steelbore notes

`jaq` is the Steelbore-canonical JSON filter. It is alias-safe for `jq`: a clean superset of the jq language in the cases that matter, with strict-numeric and arithmetic semantics that are saner than jq's. Performance is consistently 2–10× faster on real workloads.

## Recommended aliases

```nushell
alias jq = jaq
```

`safe_alias_for = ["jq"]` is set because jaq is designed to be a drop-in. The cases where it differs (rare-edge arithmetic precision, a handful of obscure builtins) are surfaced as errors rather than silent divergence.

## Recommended invocation

```sh
xh https://api.github.com/users/octocat | jaq '.name'
cat config.json | jaq '.servers[] | select(.region == "us-east")'
jaq -n '{"now": now}'             # constant input
```

## Differences from `jq`

- ~2–10× faster on representative workloads.
- Stricter arithmetic; certain corner cases fail loudly instead of producing surprising floats.
- Smaller binary, fully `no_std`-friendly core.

## Pairs with

- **xh** — the canonical Steelbore HTTP→JSON pipeline (`xh url | jaq …`).
- **rg** — pre-filter logs with `rg` before piping JSON lines into `jaq`.
