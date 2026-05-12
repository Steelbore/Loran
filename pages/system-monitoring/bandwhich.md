+++
name = "bandwhich"
category = "system-monitoring"
summary = "Per-process and per-connection bandwidth monitor for the terminal."
replaces = ["iftop", "nethogs"]
safe_alias_for = []
pairs_with = ["bottom", "procs"]
official = "https://github.com/imsnif/bandwhich"
tldr_page = "bandwhich"
written_in = "rust"
since = "bravais@0.1"
tags = ["network", "tui-friendly"]
aliases = []
+++

## Steelbore notes

`bandwhich` shows network usage broken down by process, by remote address, and by connection in a single TUI. Steelbore favours it over `iftop` (per-interface) and `nethogs` (per-process but interface-bound) because it correlates all three views simultaneously.

Requires `CAP_NET_RAW` to inspect packets; install with capabilities or run via `sudo`.

## Recommended usage

```sh
sudo bandwhich               # all interfaces
sudo bandwhich -i wlan0      # specific interface
sudo bandwhich --raw         # plain-text mode (good for piping)
```

## Differences from `iftop` / `nethogs`

- Process column shows the owning binary, not just the connection.
- TUI updates ~once per second; CPU footprint stays low.
- Optional `--raw` mode emits CSV-ish lines that compose with `awk` and `jaq`.

## Pairs with

- **bottom** — keep a `btm` and a `bandwhich` open side by side to correlate CPU spikes with traffic.
- **procs** — drill into the offending PID once `bandwhich` names it.
