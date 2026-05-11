<!--
SPDX-License-Identifier: GPL-3.0-or-later
Copyright (c) 2026 Mohamed Hammad
-->

# Lodestone — Specification v0.1 (Draft)

| Field          | Value                                                       |
|----------------|-------------------------------------------------------------|
| **Project**    | Lodestone                                                   |
| **Tagline**    | The Steelbore reference manual.                             |
| **Version**    | 0.1.0 (specification draft)                                 |
| **Date**       | 2026-04-29                                                  |
| **Author**     | Mohamed Hammad                                              |
| **Copyright**  | (c) 2026 Mohamed Hammad                                     |
| **License**    | GPL-3.0-or-later                                            |
| **Governed by**| Steelbore Standard v1.0, Steelbore SFRS v1.0.0              |

---

## 1. Purpose

Lodestone is the canonical, agent-friendly reference tool for Steelbore-based systems (Lattice, Ferrite OS, future distros). It answers three questions in one binary:

1. **What tools are available on this system?** — categorised browse of the Steelbore tool catalog.
2. **What does this tool do, and what does it replace?** — Steelbore-curated intro, then tldr page, then live `--help` as fallback.
3. **What replaces the legacy tool I know?** — `lodestone find ls` → `eza`.

Lodestone is to Steelbore what `man` is to Unix and `info` is to GNU: a system-level reference. Unlike either, it ships agent-native (`--json`, `schema`, MCP) from day one.

The metaphor: a lodestone is a naturally magnetised piece of magnetite that early sailors used as a primitive compass. Lodestone the tool is the user's compass through a Steelbore system.

---

## 2. Locked Design Decisions

These are settled and inform the rest of this spec. Listed up front so reviewers can challenge the foundations before spending time on details.

| # | Decision                                                                                                |
|---|---------------------------------------------------------------------------------------------------------|
| 1 | **Name:** Lodestone (geological — magnetised magnetite, navigational metaphor)                          |
| 2 | **Greenfield Cargo workspace.** No fork of tealdeer or tlrc. Reuse patterns, not code.                  |
| 3 | **Global registry + overlays** for the page collection (upstream / per-distro / per-user).              |
| 4 | **Category-first browsing** (mirrors the GRUB/parted help affordance the user described).              |
| 5 | **TUI default with TTY auto-detection** per SFRS §5 cascade. Pipe = `--format json`.                    |
| 6 | **Tarball update model**, mirroring tldr-pages. No git client dependency at runtime.                    |
| 7 | **Resolution order:** Steelbore intro (always) → custom page → tldr page → live `--help` → no-entry prompt. |
| 8 | **Page format:** single Markdown file with TOML frontmatter (Hugo/Zola style).                          |
| 9 | **In-process rendering** of Lodestone pages via `pulldown-cmark` + `ratatui` + `crossterm`. No `bat` for own pages. |
| 10| **`bat -pp`** as PAGER/MANPAGER for the captured `<tool> --help` subprocess only. Falls back to `cat` if `bat` absent. |
| 11| **`lodestone new <tool>`** scaffolds pages from a user-editable template, writing to the user overlay by default. |
| 12| **GPL-3.0-or-later**, SPDX headers on all source files (Steelbore Standard §4).                         |

---

## 3. Architecture

### 3.1 Cargo Workspace Layout

```
lodestone/
├── Cargo.toml                       # workspace root
├── README.md
├── LICENSE                          # GPL-3.0-or-later
├── AGENTS.md                        # generic agent context
├── CLAUDE.md                        # Claude Code-specific context
├── SKILL.md                         # capability surface for Steelbore Skills
├── CONTRIBUTING.md
├── crates/
│   ├── lodestone-cli/               # clap binary, dispatcher, exit codes
│   ├── lodestone-core/              # orchestration, resolution chain
│   ├── lodestone-index/             # TOML/Markdown index loading + cache build
│   ├── lodestone-pages/             # page parser (TOML frontmatter + body)
│   ├── lodestone-render/            # Markdown → terminal renderer
│   ├── lodestone-tldr/              # tldr tarball fetch + cache + lookup
│   ├── lodestone-tui/               # ratatui app (browse, detail, search)
│   └── lodestone-mcp/               # MCP server surface (Phase 3)
├── pages/                           # bundled fallback pages (built into binary)
│   └── …
└── xtask/                           # build/release/index-validate helpers
```

Crate-prefix naming follows mainstream Rust workspace convention. Project-level metallurgical naming applies (Lodestone passes); internal crates are implementation detail. Release codenames will follow the Standard's cast-form list (Ingot → Billet → Bloom → …).

### 3.2 Dependency Stack (canonical picks)

| Concern              | Crate                          | Rationale                                                |
|----------------------|--------------------------------|----------------------------------------------------------|
| CLI parsing          | `clap` (derive)                | SFRS canonical                                           |
| Serialization        | `serde`, `serde_json`, `toml`  | SFRS canonical                                           |
| Markdown parsing     | `pulldown-cmark`               | CommonMark, fast, no_std-friendly                        |
| TUI                  | `ratatui` + `crossterm`        | SFRS canonical                                           |
| Time                 | `jiff`                         | Microsoft Rust Guidelines preferred over chrono          |
| HTTP (tarball)       | `ureq` + `rustls`              | Lean, no async runtime needed for one-shot fetch         |
| Tar/gzip             | `tar` + `flate2`               | Standard combo                                           |
| Fuzzy search         | `nucleo-matcher`               | Used by helix; fast and well-tested                      |
| Binary cache format  | `postcard`                     | Compact, fast, schema-stable                             |
| MCP (Phase 3)        | `rmcp`                         | SFRS canonical                                           |
| Errors               | `thiserror` (lib), `anyhow` (bin) | Microsoft Rust Guidelines                              |
| Logging              | `tracing` + `tracing-subscriber`  | Per SFRS                                              |

No `tokio` in the v1 fast path. Tarball fetch is one synchronous request; everything else is local I/O. MCP server may introduce async in Phase 3 — scoped to that crate only.

---

## 4. Resolution Chain (`lodestone show <tool>`)

```
                    ┌─────────────────────────┐
                    │  lodestone show <tool>  │
                    └────────────┬────────────┘
                                 │
             ┌───────────────────┴───────────────────┐
             │ 1. Look up <tool> in resolved index   │
             │    (overlays merged, user wins)       │
             └───────────────────┬───────────────────┘
                                 │
                ┌────────────────┴────────────────┐
                │ Index hit?                      │
                └──┬─────────────────────────────┬┘
                   │ yes                         │ no
                   ▼                             ▼
        ┌──────────────────────┐   ┌──────────────────────┐
        │ Render Steelbore     │   │ Skip Steelbore intro │
        │ intro block (always) │   │                      │
        └──────────┬───────────┘   └──────────┬───────────┘
                   │                          │
                   ▼                          ▼
        ┌──────────────────────────────────────────────┐
        │ 2. Body resolution (first match wins)        │
        │   a. custom page in any overlay              │
        │   b. tldr page (if `tldr_page` set + cached) │
        │   c. spawn `<tool> --help` (sandboxed)       │
        │   d. emit no-entry diagnostic                │
        └────────────────────┬─────────────────────────┘
                             │
                             ▼
                  ┌───────────────────────┐
                  │ Render to mode        │
                  │  (TUI / text / JSON)  │
                  └───────────────────────┘
```

### 4.1 No-entry response

```
$ lodestone show widgetctl
error: no Lodestone entry for 'widgetctl'

  hint: lodestone new widgetctl --edit
        (scaffolds a page in your user overlay; opens $EDITOR)

  see also: lodestone search widget --json
```

In `--json` mode, the structured error carries the same hint (`error.hint = "lodestone new widgetctl --edit"`) per SFRS tips-thinking discipline.

### 4.2 `--help` fallback safety

When `<tool> --help` is invoked as a fallback:

- Binary path resolved via `which`-equivalent against `$PATH`. **Never** trust the user-supplied tool name as a path.
- Spawned with `argv = [tool, "--help"]`. No shell, no string interpolation.
- Environment: `PAGER="bat -pp"`, `MANPAGER="bat -pp"`, `LESS=` empty. If `bat` is not on `$PATH`, both fall back to `cat`.
- 5-second wall-clock timeout; SIGKILL on overrun.
- Try sequence on non-zero exit: `--help` → `-h` → `help` (subcommand). Capture stdout + stderr, prefer non-empty.
- Output rendered in a distinctly-themed frame (Steel Blue accent) with header: `LIVE OUTPUT — uncurated, captured from <tool> --help at <ISO 8601 UTC>`.
- In `--json`: emitted as `data.body.kind = "live_help"` with `data.body.captured_at` so agents can cache or invalidate independently.

---

## 5. Filesystem Layout

All paths follow XDG Base Directory Specification.

```
$XDG_DATA_HOME/lodestone/                  # default: ~/.local/share/lodestone
├── pages/                                 # upstream Steelbore pages (sync target)
│   ├── meta.toml                          # tarball version, fetched_at
│   └── <category>/<tool>.md
├── overlays/
│   ├── lattice/<category>/<tool>.md       # distro overlay (read-only at runtime)
│   ├── ferrite/<category>/<tool>.md
│   └── user/<category>/<tool>.md          # user overlay (writable)
└── templates/
    └── tool.md                            # editable scaffold template

$XDG_CACHE_HOME/lodestone/                 # default: ~/.cache/lodestone
├── tldr/
│   ├── pages.tar.gz                       # raw tldr tarball
│   ├── extracted/                         # unpacked tree
│   └── meta.toml                          # last-modified, etag
└── index.postcard                         # compiled index for fast startup

$XDG_CONFIG_HOME/lodestone/                # default: ~/.config/lodestone
└── config.toml                            # user preferences (active overlay, language, etc.)
```

### 5.1 Overlay precedence

Lowest to highest:

1. `pages/` — upstream Steelbore pages (synced via tarball)
2. `overlays/<active-distro>/` — distro-specific overrides (Lattice or Ferrite OS)
3. `overlays/user/` — user customisations

Active distro is resolved from `/etc/os-release` at startup (`ID=lattice`, `ID=ferrite`, …). Falls back to "generic" overlay if neither matches. Overridable via `config.toml` (`active_overlay = "lattice"`) and via `--overlay <name>` flag.

Index build merges all three layers; later layers replace earlier ones field-by-field, not record-by-record (so a user can override `summary` without re-stating `category`, `replaces`, etc.).

---

## 6. Page Format

Single Markdown file with TOML frontmatter, fenced by `+++`.

```markdown
+++
name      = "eza"
category  = "file-listing"
replaces  = ["ls"]
summary   = "Modern ls replacement. Steelbore default for file listing."
official  = "https://eza.rocks"
tldr_page = "eza"                  # key into tldr cache; omit to disable tldr lookup
language  = "rust"                 # optional metadata
since     = "lattice@0.1"          # optional: first Steelbore release shipping it
tags      = ["filesystem", "tui-friendly"]
+++

## Steelbore Notes

`eza` is the Steelbore-canonical file lister. Aliased to `ls` in the
default Lattice shell profile (Nushell). Honours `LS_COLORS` and the
Steelbore palette via the `EZA_COLORS` environment variable.

## Recommended Aliases

`````nushell
alias ls = eza --git --icons
alias ll = eza -l --git --icons
alias tree = eza --tree
`````

## Differences from `ls`

- Tree mode is built-in (`-T` / `--tree`), no need for separate `tree` binary.
- Git status integration via `--git`.
- ...
```

### 6.1 Frontmatter schema (required + optional)

| Field      | Type            | Required | Notes                                              |
|------------|-----------------|----------|----------------------------------------------------|
| `name`     | string          | yes      | Canonical binary name. Lower-kebab-case.           |
| `category` | string          | yes      | One of the values in `categories.toml`.            |
| `summary`  | string (≤120ch) | yes      | One-line description.                              |
| `replaces` | array<string>   | no       | Legacy tool names this supersedes.                 |
| `official` | URL             | no       | Upstream homepage.                                 |
| `tldr_page`| string          | no       | tldr key. Defaults to `name` if omitted; set to "" to disable. |
| `tags`     | array<string>   | no       | Free-form, surfaced in `lodestone search`.         |
| `language` | string          | no       | Implementation language. Surfaces a "🦀" badge in TUI for `rust`. |
| `since`    | string          | no       | First Steelbore release shipping this tool.        |
| `aliases`  | array<string>   | no       | Alternative spellings (e.g. ripgrep ↔ rg).         |

Validated by `lodestone-pages` at index build time. Index build fails loud on schema violations — no silent skipping.

### 6.2 Categories

Categories are first-class. The category list is a single TOML file shipped in `pages/categories.toml`:

```toml
[file-listing]
title       = "File listing"
description = "Tools that enumerate filesystem entries."

[text-search]
title       = "Text search"
description = "Tools that search file contents."

# ...
```

User overlays may add categories but cannot remove upstream ones.

---

## 7. Subcommand Surface

Noun-verb per SFRS §2 Rule 7. Verbs in the canonical set where applicable.

| Command                            | Description                                                           |
|------------------------------------|-----------------------------------------------------------------------|
| `lodestone`                        | TUI if TTY, else `lodestone list --json`. Auto-detection per SFRS §5. |
| `lodestone list`                   | List tools (filterable). Honours `--category`, `--replaces`, `--fields`. |
| `lodestone show <tool>`            | Show resolved page (Steelbore intro + body per §4 chain).             |
| `lodestone find <legacy>`          | Reverse lookup: which tool replaces `<legacy>`?                       |
| `lodestone search <query>`         | Fuzzy search across name, summary, replaces, tags.                    |
| `lodestone categories`             | List categories with counts. JSON-friendly.                           |
| `lodestone new <tool>`             | Scaffold a new page from template. `--edit` opens `$EDITOR`.         |
| `lodestone update`                 | Refresh upstream `pages/` tarball + tldr tarball. Re-builds index.    |
| `lodestone validate`               | Validate all pages against frontmatter schema. CI-friendly.           |
| `lodestone schema`                 | JSON Schema (Draft 2020-12) of own data types. SFRS §4.               |
| `lodestone describe`               | Self-description manifest for agents. SFRS §4.                        |
| `lodestone mcp`                    | Run as MCP server over stdio. Phase 3.                                |

### 7.1 Global flags (SFRS §3, identical across all Steelbore CLIs)

`--json`, `--format`, `--fields`, `--dry-run`, `--verbose`, `--quiet`, `--no-color`, `--color`, `--help`, `--version`, `--absolute-time`, `--print0`, `--yes`. No deviations.

### 7.2 `lodestone new` specifics

Two modes per the prior decision:

- **Interactive (default in TUI / TTY):** prompts for category (with autocomplete from `categories.toml`), summary, replaces; opens `$EDITOR` on the body afterward.
- **Non-interactive:** every field as a flag.

```bash
# interactive
lodestone new widgetctl

# non-interactive (scriptable, agentic)
lodestone new widgetctl \
  --category=file-listing \
  --replaces=ls,dir \
  --summary="Widget control utility" \
  --no-edit
```

Writes to `$XDG_DATA_HOME/lodestone/overlays/user/<category>/<tool>.md` by default. `--scope=upstream` writes into a user-cloned `pages/` checkout (path configured via `config.toml`) for contribution back upstream — the only place git enters the picture, and it's user-side, not bundled.

Template lives at `$XDG_DATA_HOME/lodestone/templates/tool.md`, populated on first run from a default baked into the binary, then user-editable.

---

## 8. JSON Envelope

Per SFRS §6. Example for `lodestone show eza --json`:

```json
{
  "metadata": {
    "tool": "lodestone",
    "version": "0.1.0",
    "command": "lodestone show eza",
    "timestamp": "2026-04-29T08:30:00Z"
  },
  "data": {
    "name": "eza",
    "category": "file-listing",
    "replaces": ["ls"],
    "summary": "Modern ls replacement. Steelbore default for file listing.",
    "official": "https://eza.rocks",
    "tags": ["filesystem", "tui-friendly"],
    "language": "rust",
    "intro": {
      "source": "steelbore",
      "body_md": "..."
    },
    "body": {
      "kind": "custom",
      "source_path": "/home/mh/.local/share/lodestone/pages/file-listing/eza.md",
      "body_md": "...",
      "tldr_available": true
    }
  }
}
```

`body.kind` ∈ {`custom`, `tldr`, `live_help`, `none`}, telling the agent exactly what they're looking at without reparsing.

---

## 9. Exit Codes (SFRS §4 + Lodestone-specific)

Canonical codes 0–5 unchanged. Tool-specific (6–125):

| Code | Constant                | Meaning                                                       |
|------|-------------------------|---------------------------------------------------------------|
| 6    | `INDEX_NOT_BUILT`       | Cache missing; user should run `lodestone update`.           |
| 7    | `TARBALL_FETCH_FAILED`  | Network or HTTP error during `lodestone update`.             |
| 8    | `PAGE_PARSE_ERROR`      | Frontmatter schema violation in a discovered page.           |
| 9    | `LIVE_HELP_TIMEOUT`     | `<tool> --help` exceeded 5s timeout.                         |
| 10   | `OVERLAY_WRITE_DENIED`  | `lodestone new` couldn't write to the target overlay.        |

All documented in `lodestone schema` output.

---

## 10. TUI Behaviour

- **Default view (no args, TTY):** category list (left pane) + tool list (right pane). Vim `hjkl` navigation; CUA arrow keys. `/` for fuzzy search, `?` for in-app help.
- **Detail view:** Steelbore intro block, then body, with a live indicator if `body.kind = "live_help"`. Tab-switchable to raw Markdown and frontmatter views (agent-friendly inspection).
- **Theme:** Steelbore palette only (Void Navy bg, Molten Amber primary text, Steel Blue structural, Radium Green success, Liquid Coolant info, Red Oxide error). Honours `NO_COLOR`.
- **Agent guard rail:** If `AI_AGENT=1` or `AGENT=1` is set, TUI never activates — falls back to `--format json` and warns on stderr per SFRS §5.

---

## 11. Tarball Update Mechanism

Modelled on tldr-pages, adjusted for Steelbore.

- Source: `https://pages.steelbore.org/lodestone/v1/pages.tar.gz` (CDN endpoint TBD; placeholder URL).
- Manifest: `pages.json` alongside the tarball, contains version + ETag + SHA-256.
- Fetch: `ureq` + `rustls`, `If-None-Match` against cached ETag. 304 = no work needed.
- Verify: SHA-256 against manifest before extraction.
- Extract: into `$XDG_DATA_HOME/lodestone/pages/` atomically (extract to temp dir, rename).
- Same flow runs for the tldr tarball (`https://tldr-pages.github.io/assets/tldr.zip`), into the cache dir.
- After successful extraction of either, rebuild the postcard index.
- `--dry-run` reports what would be fetched/extracted without touching disk.

---

## 12. Agent Surface

### 12.1 Context files (Day-One artifacts)

Per `steelbore-agentic-cli` §2:

- `AGENTS.md` — coding conventions, test commands, repo invariants, forbidden patterns. Generic.
- `CLAUDE.md` — references to skills (`steelbore-standard`, `steelbore-cli-standard`, `steelbore-agentic-cli`, `rust-guidelines`). Claude-specific.
- `SKILL.md` — Lodestone's own capability surface for the Steelbore Skills system to consume.
- `CONTRIBUTING.md` — human onboarding.

All present at repo root from the first commit, before `lodestone-cli` has any sub-commands.

### 12.2 MCP Surface (Phase 3)

Lodestone's full sub-command count (≈12) is borderline for SFRS §2 Rule 8's MCP threshold. We ship MCP because the read-only ones (`list`, `show`, `find`, `search`, `categories`) are unusually high-value for agents discovering what tools exist on a Steelbore system. Lazy-loading discipline per `steelbore-agentic-cli` §6: `tools/list` advertises names + capability tags only; full schemas come from `tools/get`.

### 12.3 Tips-thinking error catalog

Every error code from §9 has a runnable `hint`. Examples:

| Code                  | Example `hint`                                                               |
|-----------------------|------------------------------------------------------------------------------|
| `INDEX_NOT_BUILT`     | `lodestone update`                                                           |
| `NOT_FOUND`           | `lodestone search <query> --json` (with the user's query interpolated)       |
| `PAGE_PARSE_ERROR`    | `lodestone validate --json` (returns the offending file + line)              |
| `OVERLAY_WRITE_DENIED`| `mkdir -p $XDG_DATA_HOME/lodestone/overlays/user && lodestone new <tool>`    |
| `LIVE_HELP_TIMEOUT`   | `lodestone new <tool> --edit` (write a curated page instead)                 |

---

## 13. Steelbore Standard §13 Compliance Checklist

| § | Requirement                            | Status / Note                                                                       |
|---|----------------------------------------|-------------------------------------------------------------------------------------|
| 2 | Metallurgical naming                   | ✓ "Lodestone" (geological/magnetic). Releases will use Ingot/Billet/Bloom.          |
| 3.1 | Memory safety                        | ✓ Rust throughout. `rust-guidelines` skill to be loaded at implementation time.     |
| 3.2 | Performance / concurrency designed-in | ✓ Sync where appropriate; async confined to MCP crate. Index build benched.         |
| 3.3 | Hardened security; PQC                | ✓ `rustls` for tarball fetch (PQC-ready via hybrid KEMs in upstream roadmap). No crypto subsystem of our own. |
| 4 | GPL-3.0-or-later + SPDX                | ✓ All `.rs` and Cargo.toml files. Pages (Markdown) are documents → exempt.          |
| 5.1 | POSIX-compliant CLI                  | ✓ SFRS-compliant; default output is POSIX-safe.                                     |
| 6 | PFA: no tracking, minimal perms, local | ✓ No telemetry. Filesystem + outbound HTTPS to tarball CDN only. All data local.   |
| 7 | CUA + Vim bindings                     | ✓ Both schemes in TUI.                                                              |
| 8 | Steelbore palette; Void Navy bg        | ✓ TUI uses palette tokens only.                                                     |
| 9 | FOSS fonts                             | N/A — terminal app, uses user's terminal font. Docs use Share Tech Mono / Inconsolata. |
| 10| Material Design / WCAG 2.1 AA          | N/A for v1 (no GUI). WCAG-AA contrast already satisfied by palette per §8.          |
| 11| ISO 8601 / UTC / 24h / metric          | ✓ All timestamps in JSON envelope and `live_help` captures.                         |

---

## 14. Phasing

| Phase   | Codename | Scope                                                                                       |
|---------|----------|---------------------------------------------------------------------------------------------|
| Phase 1 | Ingot    | Workspace + global flags + JSON envelope + `list` / `show` / `find` / `search` / `categories` + index from bundled `pages/`. Text-mode only. |
| Phase 2 | Billet   | Tarball update (upstream + tldr) + overlays + `--help` fallback + TUI + `lodestone new` + `validate`. |
| Phase 3 | Bloom    | MCP surface + `lodestone schema` JSON-Schema-ified for Anthropic/OpenAI/Gemini/MCP function-calling + Lattice/Ferrite OS overlay catalogs in tree. |

Ingot is shippable on its own as a useful binary, even before tarball/overlay machinery exists. Billet is the user-visible 1.0 milestone. Bloom is the agentic completion.

---

## 15. Open Questions for v0.2

1. **Tarball CDN host** — `pages.steelbore.org` is a placeholder. Where does the upstream pages tree actually live, and who runs the publisher pipeline?
2. **Overlay distribution** — does the Lattice overlay live in the Lodestone repo or in the Lattice repo? (Coupling vs. cohesion trade-off; I lean toward Lattice repo, surfaced into Lodestone via the tarball pipeline.)
3. **i18n** — frontmatter has no `language` field for translations. tldr-pages handles this with `pages.<lang>/`. Defer to v1.x or design in now?
4. **Categories: flat or nested?** — current spec is flat. Nested (`system/file-listing`, `system/text-search`) might scale better as the catalog grows, but complicates the UX. Defer until the catalog hits ~50 entries.
5. **Page signing** — should upstream pages be signed (Sequoia / minisign) and verified at extract time? Aligns with the §3.3 hardened-security priority but adds a dependency.

---

*Forged in Steelbore.*
