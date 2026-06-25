+++
name = "btmgmt"
category = "bluetooth"
summary = "BlueZ management-API client. Non-interactive adapter and device control for scripts and recovery."
replaces = []
safe_alias_for = []
pairs_with = ["bluetoothctl"]
official = "https://github.com/bluez/bluez"
written_in = "c"
since = "bravais@0.1"
tags = ["bluetooth", "bluez"]
aliases = []
+++

## Spacecraft Software notes

`btmgmt` talks to the BlueZ management API directly rather than through the D-Bus agent layer that `bluetoothctl` uses. That makes it the right tool when the higher-level stack is wedged — toggling an adapter, forcing it discoverable, or driving setup from a non-interactive script.

## Recommended usage

```sh
btmgmt info                              # adapters and their current settings
btmgmt power on
btmgmt discov on                         # make the adapter discoverable
btmgmt find                              # scan for nearby devices
btmgmt pair -c <type> AA:BB:CC:DD:EE:FF
```

`btmgmt` usually needs root — it holds the management socket. When pairing works here but not in `bluetoothctl`, an agent/policy issue in the session layer is the likely cause.

## Pairs with

- **bluetoothctl** — the interactive, agent-backed client for day-to-day pairing; `btmgmt` is the escape hatch beneath it.
