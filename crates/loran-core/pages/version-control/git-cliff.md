+++
name = "git-cliff"
category = "version-control"
summary = "Conventional-commit changelog generator. Configurable templates, no JS runtime."
replaces = ["standard-version", "conventional-changelog"]
safe_alias_for = []
pairs_with = ["lazygit", "delta"]
official = "https://git-cliff.org"
tldr_page = "git-cliff"
written_in = "rust"
since = "bravais@0.1"
tags = ["git", "release"]
aliases = []
+++

## Spacecraft Software notes

`git-cliff` is Spacecraft Software's preferred changelog generator. It reads conventional-commit messages from the git log, applies a template you control, and emits a Markdown changelog ready for release notes. Unlike the Node.js ecosystem alternatives it replaces, it is a single static binary with no runtime dependency.

Drop a `cliff.toml` in the repo root, then:

```sh
git-cliff --tag v0.2.0 --output CHANGELOG.md
git-cliff --latest                       # only the latest release
git-cliff --unreleased --strip header    # for PR descriptions
```

## Recommended setup

`cliff.toml` snippet that mirrors the Loran release style:

```toml
[changelog]
header = "# Changelog\n"
body = """
## {{ version | default(value="Unreleased") }}{% if timestamp %}  ({{ timestamp | date(format="%Y-%m-%d") }}){% endif %}
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group }}
{% for commit in commits %}- {{ commit.message | upper_first }}
{% endfor %}
{% endfor %}
"""

[git]
conventional_commits = true
filter_unconventional = true
```

## Differences from the JS tools

- Single statically-linked binary, ships in the project's own release artifacts.
- Tera templates, not Handlebars; configuration is one TOML file.
- Honours `--unreleased` so you can preview the PR's changelog effect before tagging.

## Pairs with

- **lazygit** — craft tidy conventional-commit messages while you're staging.
- **delta** — review the resulting diff and the changelog with the same theming.
