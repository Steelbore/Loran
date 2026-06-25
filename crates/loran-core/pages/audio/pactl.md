+++
name = "pactl"
category = "audio"
summary = "PulseAudio control-protocol client. Drives sinks, sources, and modules on PulseAudio or pipewire-pulse."
replaces = []
safe_alias_for = []
pairs_with = ["wpctl"]
official = "https://www.freedesktop.org/wiki/Software/PulseAudio/"
tldr_page = "pactl"
written_in = "c"
since = "bravais@0.1"
tags = ["audio", "pulseaudio"]
aliases = []
+++

## Spacecraft Software notes

`pactl` speaks the PulseAudio control protocol. On a modern Spacecraft Software desktop that protocol is usually served by `pipewire-pulse`, so `pactl` and `wpctl` manipulate the *same* audio graph from two vocabularies. Prefer `wpctl` for native PipeWire work; reach for `pactl` when a tool, script, or upstream guide is written in PulseAudio terms.

## Recommended usage

```sh
pactl info                                   # server, default sink/source
pactl list short sinks                       # enumerate outputs (name + index), tab-separated
pactl set-sink-volume @DEFAULT_SINK@ +5%     # relative volume change
pactl set-sink-mute   @DEFAULT_SINK@ toggle
pactl set-default-sink <NAME>                # route the default output
pactl set-sink-input-volume <INPUT> 80%      # per-application volume
```

Use the `list short` forms in scripts — they are tab-separated and stable; the long forms are for humans.

## Pairs with

- **wpctl** — the native PipeWire controller for the same graph; canonical for `set-default` and routing on PipeWire.
