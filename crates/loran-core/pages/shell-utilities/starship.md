+++
name = "starship"
category = "shell-utilities"
summary = "Cross-shell prompt. One TOML config drives Bash, Zsh, Fish, Nushell, PowerShell, Ion."
replaces = ["powerline", "powerline-shell", "p10k"]
safe_alias_for = []
pairs_with = ["nu", "direnv"]
official = "https://starship.rs"
tldr_page = "starship"
written_in = "rust"
since = "bravais@0.1"
tags = ["prompt", "shell"]
aliases = []
+++

## Spacecraft Software notes

`starship` is the Spacecraft Software-canonical shell prompt. One `~/.config/starship.toml` works across every shell the team uses (Nushell, Ion, PowerShell, Bash, Zsh, Fish), so the prompt stays consistent when you switch shells or pair-program.

## Recommended setup

```nushell
# ~/.config/nushell/config.nu
$env.STARSHIP_SHELL = "nu"
$env.PROMPT_INDICATOR = ""
$env.PROMPT_COMMAND = { ||
    let width = (term size).columns
    starship prompt --cmd-duration $env.CMD_DURATION_MS $"--status=($env.LAST_EXIT_CODE)" --terminal-width=$width
}
```

```sh
# Bash / Zsh
eval "$(starship init bash)"   # or zsh / fish / pwsh
```

## Configuration highlights

- Detects 30+ tool versions (Rust, Go, Node, Python, …) and surfaces the active one.
- Optional `[character] success_symbol` swap for the Spacecraft Software Molten Amber prompt arrow.
- `cmd_duration` annotates long-running commands so you notice when something stalled.

## Pairs with

- **nu** — first-class Nushell support; the same prompt config Just Works.
- **direnv** — when you `cd` into a project with `.envrc`, `starship` shows the activated environment.
