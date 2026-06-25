+++
name = "wpctl"
category = "audio"
summary = "WirePlumber control. Spacecraft Software default for PipeWire volume, default sinks, and routing."
replaces = []
safe_alias_for = []
pairs_with = ["pactl"]
official = "https://pipewire.pages.freedesktop.org/wireplumber/"
tldr_page = "wpctl"
written_in = "c"
since = "bravais@0.1"
tags = ["audio", "pipewire"]
aliases = []
+++

## Spacecraft Software notes

`wpctl` is the WirePlumber session-manager controller and the Spacecraft Software-canonical way to drive audio on a PipeWire stack. It speaks to PipeWire natively — no PulseAudio shim — so it sees every node, sink, and source the graph exposes. Reach for `pactl` only when a tool or muscle-memory expects PulseAudio command shapes.

## Recommended usage

```sh
wpctl status                              # the full graph: sinks, sources, streams
wpctl get-volume @DEFAULT_AUDIO_SINK@     # current default-output volume
wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+ # nudge the default output up 5%
wpctl set-mute   @DEFAULT_AUDIO_SINK@ toggle
wpctl set-default <ID>                    # make a sink/source the default (IDs from `status`)
```

`@DEFAULT_AUDIO_SINK@` / `@DEFAULT_AUDIO_SOURCE@` are stable aliases — bind them to media keys instead of hard-coding node IDs, which renumber across reboots.

## Pairs with

- **pactl** — the PulseAudio-compatible control surface over the same graph; use it when a script or guide is written against `pactl`.
