+++
name = "dog"
category = "networking"
summary = "Modern DNS lookup tool — dig with sane defaults and JSON output."
replaces = ["dig", "host", "nslookup"]
safe_alias_for = []
pairs_with = ["xh"]
official = "https://github.com/ogham/dog"
tldr_page = "dog"
written_in = "rust"
since = "bravais@0.1"
tags = ["dns", "networking"]
aliases = []
+++

## Steelbore notes

`dog` is the Steelbore-canonical DNS lookup tool. It is to `dig` what `bat` is to `cat`: same job, much better defaults — colour, structured output, IPv4/IPv6 parity, DNS-over-TLS and DNS-over-HTTPS support, JSON mode (`--json`) suitable for piping into `jaq`.

`safe_alias_for` is empty: scripts that parse `dig +short` output by position will not survive being repointed at `dog`. Use `dog` interactively and keep `dig` for legacy parsing.

## Recommended invocation

```sh
dog example.com                  # A records
dog example.com AAAA             # IPv6
dog example.com MX TXT           # multiple record types
dog example.com --https          # DoH against the default resolver
dog example.com @1.1.1.1         # specify resolver
dog example.com --json | jaq .   # structured output
```

## Differences from `dig`

- Coloured, columnar output by default.
- Multi-record-type queries in one invocation.
- DNS-over-TLS (`--tls`) and DNS-over-HTTPS (`--https`) built in.
- `--json` mode for scripting.

## Pairs with

- **xh** — the natural follow-up after a `dog` lookup is an HTTP request; both surface JSON for piping into `jaq`.
