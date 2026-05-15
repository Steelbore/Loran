+++
name = "hyperfine"
category = "shell-utilities"
summary = "Benchmarking tool for CLI commands. Warmup runs, statistical analysis, JSON export."
replaces = ["time"]
safe_alias_for = []
pairs_with = ["just", "miller"]
official = "https://github.com/sharkdp/hyperfine"
tldr_page = "hyperfine"
written_in = "rust"
since = "bravais@0.1"
tags = ["benchmark", "performance"]
aliases = []
+++

## Spacecraft Software notes

`hyperfine` is the Spacecraft Software-canonical command-line benchmark. It runs warmup iterations to settle filesystem caches, then collects ≥10 timed runs, then reports mean / stddev / min / max with an outlier warning. The output beats `time` because the variance you actually need is right there.

`safe_alias_for` is empty: `time` is a shell keyword that already aliases something specific in most shells; `hyperfine` is a separate binary with different output semantics.

## Recommended usage

```sh
hyperfine 'cargo test --workspace'
hyperfine --warmup 3 'rg pattern' 'grep -r pattern'    # head-to-head
hyperfine --parameter-list n 10,100,1000 'fd . -d {n}'  # parameterised sweep
hyperfine --export-json bench.json 'just ci'           # machine-readable
```

## Why "warmup" matters

The first run of any command pays page-fault, dentry-cache, and JIT-warmup costs that are not representative of steady-state performance. `--warmup 3` runs three throwaway iterations before the measured ones; the timing you read is what users will actually experience.

## Differences from `time`

- Multiple runs with statistics, not a single shot.
- Head-to-head mode benchmarks two commands and reports the ratio.
- `--export-json` / `--export-markdown` for CI dashboards and PR comments.

## Pairs with

- **just** — wrap a `bench` recipe so the whole team benchmarks the same way.
- **miller** — analyse `--export-csv` output across many runs over time.
