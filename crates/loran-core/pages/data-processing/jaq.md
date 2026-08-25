+++
name = "jaq"
category = "data-processing"
summary = "Fast Rust jq clone. Close to jq, but not alias-safe: see Pathfinder."
replaces = ["jq"]
safe_alias_for = []
pairs_with = ["xh", "rg", "pathfinder"]
official = "https://github.com/01mf02/jaq"
tldr_page = "jaq"
written_in = "rust"
since = "bravais@0.1"
tags = ["json", "filter"]
aliases = []
+++

## Spacecraft Software notes

`jaq` is the Spacecraft Software-canonical JSON filter: safe Rust, a much smaller
binary, and consistently faster than jq on real workloads.

It is **not** a drop-in for `jq`, despite being widely described as one. Measured
against jq 1.8.1: nine jq command-line flags make jaq exit 2 with
`unknown flag`, twenty-two jq builtins are `undefined filter`, and jaq does not
auto-vivify. Reach for jaq directly when you are writing a new filter; use
[Pathfinder](https://Pathfinder.SpacecraftSoftware.org/) when you need existing
jq scripts to keep working.

## Do not alias it to `jq`

`safe_alias_for` is deliberately empty. `alias jq = jaq` looks fine until it
fails at runtime, and the most common way it fails is not an exotic builtin:

```sh
echo 'null' | jq  '.a.b = 1'   # {"a":{"b":1}}
echo 'null' | jaq '.a.b = 1'   # Error: cannot use null as iterable
```

jq creates the missing containers along an assignment path; jaq does not. Every
"build the object as you go" idiom — `.metadata.labels.x = "y"` on a document
without `.metadata`, `reduce … (null; .[$k] = …)` — breaks. So do `-s`/`inputs`
across several input files (jq treats them as one stream; jaq runs the program
once per file), and `jq -c --tab` (jq resolves output-format flags last-wins;
jaq lets `-c` win regardless).

## Recommended invocation

```sh
xh https://api.github.com/users/octocat | jaq '.name'
cat config.json | jaq '.servers[] | select(.region == "us-east")'
jaq -n '{"now": now}'             # constant input
```

## Differences from `jq`

- **No auto-vivification.** The one that breaks scripts; see above.
- Twenty-two jq builtins are missing, including `tostream`, `fromstream`,
  `IN`, `INDEX`, `JOIN`, `builtins` and `input_filename`.
- No `--stream`; also no `-a`, `--seq`, `--jsonargs`, `--unbuffered`, `-b`.
- `"a" * 0` is `null` (jq: `""`); `1 / 0` is `Infinity`, which is invalid JSON
  on stdout (jq errors).
- Error text and exit-code-adjacent diagnostics differ, though the exit **codes**
  themselves match jq in every case tested.
- Faster, smaller binary, `no_std`-friendly core, and literal numbers stay exact
  through arithmetic where jq falls back to a double.

Pathfinder's `doc/DIVERGENCES.md` carries the full measured table.

## Pairs with

- **pathfinder** — a jq-compatible shim over jaq. Install it as `jq` and existing
  scripts keep working; it translates the command line, supplies the missing
  builtins, and reports the handful of divergences it cannot repair.
- **xh** — the canonical Spacecraft Software HTTP→JSON pipeline (`xh url | jaq …`).
- **rg** — pre-filter logs with `rg` before piping JSON lines into `jaq`.
