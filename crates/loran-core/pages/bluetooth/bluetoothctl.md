+++
name = "bluetoothctl"
category = "bluetooth"
summary = "BlueZ interactive client. Spacecraft Software default for scanning, pairing, and connecting devices."
replaces = []
safe_alias_for = []
pairs_with = ["btmgmt"]
official = "https://github.com/bluez/bluez"
tldr_page = "bluetoothctl"
written_in = "c"
since = "bravais@0.1"
tags = ["bluetooth", "bluez"]
aliases = []
+++

## Spacecraft Software notes

`bluetoothctl` is the interactive front-end to BlueZ over D-Bus and the Spacecraft Software-canonical tool for everyday pairing. It runs as a REPL, but every sub-command also works as a one-shot argument — which is what makes it scriptable.

## Recommended usage

```sh
bluetoothctl power on
bluetoothctl scan on                     # discover; `scan off` to stop
bluetoothctl devices                     # list known devices (MAC + name)
bluetoothctl pair    AA:BB:CC:DD:EE:FF
bluetoothctl trust   AA:BB:CC:DD:EE:FF   # auto-reconnect on future boots
bluetoothctl connect AA:BB:CC:DD:EE:FF
```

For a guaranteed-clean pairing, `remove` the device first, then walk `scan on` → `pair` → `trust` → `connect`.

## Pairs with

- **btmgmt** — the lower-level, non-interactive management client; use it for adapter resets and scripted setup where a REPL is awkward.
