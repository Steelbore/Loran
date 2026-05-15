<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# Ambiguous Review — Steelbore → Spacecraft Software rename

Generated alongside the mechanical rename pass described in
`/steelbore/steelbore/spacecraft-software-rename-prompt.md`.

## Unresolved ambiguities (left as-is for human review)

**None.** Every `Steelbore` occurrence in this repository had unambiguous
brand semantics — there are no `Steelbore OS` / `Steelbore OS Bravais`
/ `Steelbore OS Lattice` references in Loran, and no
`github.com/UnbreakableMJ/…` URLs to protect. Every match was the
umbrella-brand sense and was renamed mechanically.

## Judgment calls — resolved

All five calls have been confirmed or revised per maintainer decision
(see commit log alongside this file).

### 1. Domain → subdomain vs. subpath — **confirmed subdomain**

`https://Loran.Steelbore.com/` → `https://Loran.SpacecraftSoftware.org/`
(subdomain form, as chosen by the mechanical pass). The alternative
subpath form (`https://SpacecraftSoftware.org/loran/`) was rejected.
Files touched: `Cargo.toml`, `crates/loran/src/version.rs`,
`crates/loran/src/envelope.rs`, `crates/loran-core/src/pipeline.rs`,
`crates/loran/src/cmd/schema.rs`, `crates/loran/src/cli.rs`,
`crates/loran/tests/version_help.rs`, README.md, four planning docs.

### 2. `SYNTH_CATEGORY` slug — **revised to `spacecraft-cli`**

`crates/loran-index/src/describe.rs:44`

```rust
pub const SYNTH_CATEGORY: &str = "spacecraft-cli";
```

Was `"steelbore-cli"`, briefly `"spacecraft-software-cli"` after the
mechanical pass. Final value matches the skill-ID hyphenation style
used elsewhere. Test assertions at `crates/loran/tests/phase3.rs:119,150`
updated in lockstep. This is a public JSON envelope shape change vs.
v0.3.0 (`loran describe --json` `category` field) — external consumers
that keyed off `"steelbore-cli"` will need to switch. No legacy alias
is being maintained.

### 3. Grammar artifacts — **clean**

- `README.md:223` — reads "Loran is a personal hobby project under
  Spacecraft Software." Stranded article dropped during the touch-up
  pass.
- `CONTRIBUTING.md:70` — reads "Spacecraft Software values rigour over
  speed and correctness over cleverness…" Handled by the mechanical
  rule, no further edits needed.

### 4. Bundled-page user-facing copy — **swept clean**

All 26 curated pages under `crates/loran-core/pages/` carry the
`## Spacecraft Software notes` heading; opening sentences use
"Spacecraft Software-canonical" / "canonical Spacecraft Software
replacement" / etc. No `steelbore` residue remains anywhere outside
this review file. The bundled-page bytes are baked into the binary at
build time (`build.rs`) and surface in `loran show <tool>` output, so
the rebuild after these edits is the final source of truth.

### 5. Underscore-bounded residual — **fixed**

`crates/loran-tui/src/theme.rs:82` — `fn full_palette_uses_steelbore_tokens`
→ `fn full_palette_uses_spacecraft_software_tokens`. Renamed by hand
because `\bsteelbore\b` did not match `_steelbore_` (`_` is a word
character).

## Out-of-scope items observed (not renamed)

- `lodestone-spec-v0.1.md` — pre-Loran-rename historical document.
  Steelbore references were renamed alongside the rest of the repo per
  the prompt's "walk every text file" instruction. The Lodestone /
  Loran rename is independent and was not touched.
- `crates/loran-tldr/` — placeholder crate with `publish = false`;
  contains no Steelbore references.
- `Cargo.lock` — no Steelbore references; unchanged.
- The `Steelbore` literal in the test snapshot files
  (`crates/loran/tests/snapshots/snapshots__snapshot_*.snap`) was
  regenerated via `INSTA_UPDATE=always cargo test -p loran --test
  snapshots` after the bundled-page copy changed.

## Out-of-band manual tasks for the maintainer

These cannot be done by a code-walking agent:

- [x] Rename the GitHub organisation `github.com/Steelbore` →
      `github.com/Spacecraft-Software` (GitHub's org-rename UI handles
      the redirect for existing clones). **Done** — `git remote -v`
      shows the new URL.
- [x] Update the local clone's `[remote "origin"]` URL in
      `.git/config`. **Done.**
- [ ] Register / DNS-configure `SpacecraftSoftware.org` and
      `Loran.SpacecraftSoftware.org` (or whichever URL shape is
      adopted from §1 above).
- [ ] Update the `homepage` and `repository` metadata for already-
      published crates on crates.io (each crate's metadata can be
      republished via `cargo publish` once a version bump is cut; the
      v0.3.0 metadata on crates.io still points to the old URLs).
- [ ] Update GitHub Release notes attached to `v0.1.0-ingot`,
      `v0.2.0-billet`, `v0.3.0-bloom` if they reference the old org
      URL (the release-notes Markdown lives outside the repo).
- [ ] Update any external project-registry entries (Lobste.rs,
      crates.io account profile, personal site) that point at the
      old org or domain.
- [ ] Update minisign trust policy / `OPERATIONS.md` channels if key
      rotation is announced via a Steelbore-branded mailing list or
      Matrix room that's also being renamed.
