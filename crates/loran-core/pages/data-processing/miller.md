+++
name = "miller"
category = "data-processing"
summary = "awk for CSV / TSV / JSON / Parquet — name-keyed, schema-aware, streaming."
replaces = ["awk", "csvkit"]
safe_alias_for = []
pairs_with = ["jaq", "rg"]
official = "https://miller.readthedocs.io"
tldr_page = "mlr"
written_in = "go"
since = "bravais@0.1"
tags = ["tabular", "json", "csv"]
aliases = ["mlr"]
+++

## Spacecraft Software notes

Where `jaq` owns JSON, `miller` (`mlr`) owns tabular data. It speaks CSV, TSV, JSONL, and Parquet natively and lets you reference fields by name rather than positional index — no more `awk -F, '{print $7}'` fragility when columns reorder.

```sh
mlr --csv cut -f name,role employees.csv
mlr --icsv --opprint stats1 -a mean -f salary -g department employees.csv
mlr --c2j --json cat employees.csv         # CSV → JSON
mlr --j2c --csv cat data.json              # JSON → CSV
```

## Recommended usage

```sh
mlr --csv head -n 5 file.csv
mlr --csv filter '$status == "active"' employees.csv
mlr --csv put '$total = $price * $qty' orders.csv
```

## Differences from `awk`

- Field references are by name (`$status`), not position (`$3`).
- Format-aware: `--csv` / `--tsv` / `--json` / `--parquet` handle quoting and headers.
- Verbs are composable on the same pipeline: `cat then cut then filter then stats1`.

## Pairs with

- **jaq** — for nested JSON; `mlr` for flat-tabular.
- **rg** — to find which file has the data, then `mlr` to query it.
