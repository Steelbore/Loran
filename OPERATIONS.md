<!--
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 Mohamed Hammad
-->

# Loran operations runbook

Operational procedures for Loran maintainers. This document is the
canonical reference for the **trust-pinned publisher key** lifecycle —
how the upstream catalog is built and signed (§2), how to plan a normal
rotation, run a parallel-key transition window, and recover from an
emergency compromise.

It resolves **PRD Open Question 1** ("How do we rotate the publisher's
signing key?") and operationalises **Spec §2 decision #6** (minisign +
ed25519 over a trust-pinned key baked into the binary).

## Audience

- Loran release engineers: every section.
- Spacecraft Software Operations: §4 (emergency rotation).
- Downstream operators (Bravais OS, Ferrite OS, …): §1.4 reading list
  so an in-the-wild compromise is recognisable from the user side.

## 1. Trust-pinned-key model

### 1.1 What is baked into the binary

Every Loran release embeds the trust-pinned minisign public key(s)
directly in the binary via `include_str!`. The set is exposed as the
`UpdateOpts::public_keys: Vec<String>` field in `loran-core` and is
consulted by `signing::verify_any` during every `loran update`.

For releases that ship a single active publisher key, the slice is
length-one. For rotation windows, the slice contains **both** the
outgoing and incoming keys — `verify_any` accepts a tarball whose
signature matches *any* of them (§3.2).

### 1.2 Why baked-in, not fetched

A trust-pinned key that's fetched at runtime trades cryptographic
guarantee for first-use trust. Loran refuses that trade: the only way
to change the trust root is to ship a new Loran release. The
consequence — and the reason §3 and §4 exist — is that the rotation
procedure must be planned well before the trust root has to change.

### 1.3 Key inventory

| Constant in `loran-core::pipeline` | Used for           | Notes                            |
|------------------------------------|--------------------|----------------------------------|
| `PUBLISHER_PUBLIC_KEY`             | Upstream pages tarball | Placeholder; replace with the real key at first launch (§2.4) |

When parallel keys are active the `default_publisher` constructor
returns both via the `LORAN_PAGES_PUBLIC_KEY` env var (comma-separated)
or by editing the embedded constant set to a `Vec` of base64 strings.

### 1.4 What downstream operators should watch for

- An unexpected `TARBALL_VERIFY_FAILED` (exit 11) from `loran update`
  is the first signal of a key mismatch. Compare the running Loran
  version against the [release advisories](#6-release-advisories) —
  a parallel-key window means a stale Loran has been left running past
  the cut-over.
- A signed Spacecraft Software release announcement is the only authoritative
  source for the active publisher key. Operators should not import
  keys obtained any other way.

## 2. Producing & publishing the pages tarball

The upstream catalog is authored in the **`loran-pages`** content repo
(<https://github.com/Spacecraft-Software/loran-pages>), not in this
repository. That repo's CI is the **producer** side of the
`loran update` mechanism; this section documents how it builds and signs
the artifacts that `update_pages` consumes.

### 2.1 Artifacts and endpoint

`scripts/build-pages.sh` in `loran-pages` produces three files under
`dist/`, and `publish.yml` uploads them to the rolling `pages-latest`
GitHub Release:

| Asset | Role |
|-------|------|
| `pages.tar.gz` | gzip tarball; category dirs at the archive root, extracted verbatim into `$XDG_DATA_HOME/loran/pages/`. |
| `pages.json` | manifest — `{version, etag, sha256, size}`. |
| `pages.tar.gz.minisig` | detached minisign signature over `pages.tar.gz`. |

They are served from
`https://github.com/Spacecraft-Software/loran-pages/releases/latest/download/<asset>`
— exactly the URLs baked into
`loran-core::pipeline::PUBLISHER_PAGES_{MANIFEST,TARBALL,SIG}_URL`. The
build is deterministic (sorted entries, pinned `mtime`, `gzip -n`), so
unchanged content yields identical bytes; the published asset's ETag
stays stable across rebuilds and keeps client `304`s cheap.

### 2.2 Where the signing key lives

The publisher **secret** key is held in the Spacecraft Software release
vault (§1.1) and mirrored into the `loran-pages` repository's Actions
secrets so `publish.yml` can sign unattended:

- `MINISIGN_SECRET_KEY` — the minisign secret-key file contents.
- `MINISIGN_PASSWORD` — its password (may be empty).

The matching **public** key is baked into the Loran binary as
`PUBLISHER_PUBLIC_KEY` (§1.1). The two are always a pair: changing
either is a key rotation (§3 / §4), never an isolated edit.

### 2.3 Publish cadence

`publish.yml` runs on every push to `loran-pages` `main` that touches
`pages/**` (and on manual dispatch). It validates (`loran validate
pages/`), packs, signs, and rolls the `pages-latest` release. Clients
pick the change up on their next `loran update` — manual, or the opt-in
auto-update once the cached catalog passes its staleness interval.

### 2.4 First-launch checklist

The pipeline currently ships a development **placeholder** key (§1.3,
§5). To go live:

1. Generate the production keypair:
   `nix shell nixpkgs#minisign -c minisign -G`
2. Store the secret key in the release vault; add `MINISIGN_SECRET_KEY`
   and `MINISIGN_PASSWORD` to the `loran-pages` repo's Actions secrets.
3. Replace `PUBLISHER_PUBLIC_KEY` in `loran-core::pipeline` with the new
   public key (and keep `signing::tests::TEST_PUBLIC_KEY` distinct —
   §5).
4. Cut a Loran release carrying the real key, so installed clients trust
   the publisher (the baked placeholder never validated a real tarball,
   so there is no breakage for existing users).
5. Trigger `publish.yml` (push to `loran-pages`, or manual dispatch) to
   cut the first signed `pages-latest` release.
6. Verify end-to-end against a clean `$XDG_DATA_HOME`: `loran update`,
   then `loran show <a-tool>`.

## 3. Normal rotation procedure

Loran rotates the publisher key on a **planned annual cadence** as the
default. The rotation is announced in advance, executed during a
parallel-key transition window, and concluded with a release that
drops the outgoing key.

### 3.1 Pre-rotation (T-30 days)

1. Generate the new keypair with the upstream-blessed `minisign`
   binary (Nix-provided to avoid supply-chain drift):
   `nix shell nixpkgs#minisign -c minisign -G`
2. Store the secret key in the Spacecraft Software release vault. The public
   key is committed verbatim to a new release-notes draft.
3. Open a tracking issue. Announce the planned cut-over date and the
   parallel-key window length (≥14 days recommended).

### 3.2 Parallel-key window (T-0 → T+14d)

1. Cut a Loran release whose embedded `PUBLISHER_PUBLIC_KEY` array
   contains **both** the outgoing and incoming public keys. The
   `verify_any` primitive accepts a tarball signed by either.
2. Sign every new upstream tarball with the **incoming** secret key
   from the moment that release ships.
3. The outgoing key remains a valid trust root for the duration of
   the window so that operators who haven't yet upgraded Loran
   continue to receive updates.
4. Publish a release advisory naming the parallel-key window's start
   and the announced cut-over date.

### 3.3 Cut-over (T+14d)

1. Cut a follow-up Loran release whose embedded
   `PUBLISHER_PUBLIC_KEY` array contains **only** the incoming key.
2. Announce the cut-over via the same release-advisory channel; the
   outgoing key is now retired.
3. Tarballs signed by the outgoing key will fail
   `verify_any → SignError::Mismatch` from the cut-over release
   forward, which surfaces as exit code 11 (`TARBALL_VERIFY_FAILED`).

### 3.4 Post-rotation

1. Destroy the outgoing secret key from the release vault. The
   advisory should reference the destruction timestamp.
2. Schedule the next rotation's tracking issue.

## 4. Emergency rotation (compromise)

Emergency rotation is used when the outgoing key is known or
strongly suspected to be compromised. The procedure differs from a
normal rotation in two ways: there is no parallel-key window, and
the advisory is published before the new release ships.

### 4.1 Containment (T-0, hour 1)

1. Pause every signing pipeline. Do **not** sign any further tarball
   with the compromised key.
2. Publish a security advisory naming the compromised key and
   instructing operators to halt `loran update` until further
   notice.
3. Open an incident channel; assign an incident commander.

### 4.2 Replacement (hours 2-24)

1. Generate a new keypair (§3.1).
2. Cut an emergency Loran release whose embedded
   `PUBLISHER_PUBLIC_KEY` array contains **only** the new key. The
   compromised key is omitted — every tarball it signed becomes
   un-verifiable.
3. Sign a fresh upstream tarball with the new key.
4. Update the advisory with the new release version and the new
   public key fingerprint.

### 4.3 Recovery (days 2-7)

1. Operators upgrade Loran to the emergency release; this is the
   only safe way to obtain a tarball signed by the new key.
2. Investigate the root cause of the compromise; publish a
   post-incident report.
3. Destroy the compromised secret key if it hasn't already been;
   verify backups don't retain it.

## 5. Test fixtures and key reuse

- `crates/loran-core/src/signing.rs::tests::TEST_PUBLIC_KEY` is a
  test-only key with a documented purpose. **It must never be the
  active publisher key.** Phase 2 reuses the same string as the
  placeholder `PUBLISHER_PUBLIC_KEY` precisely so an operator who
  accidentally runs `loran update` against the placeholder URL gets a
  clean diagnostic rather than a surprising decode failure.
- At first launch (§2.4), the release engineer must swap
  `PUBLISHER_PUBLIC_KEY` to the real key and update
  `signing::tests::TEST_PUBLIC_KEY` to remain distinct.

## 6. Release advisories

All publisher-key advisories — planned and emergency — are published
to:

1. The Loran GitHub repository's `Security` tab.
2. The Spacecraft Software release-notes feed.
3. The Bravais OS and Ferrite OS distribution advisory channels for
   downstream amplification.

Each advisory must include:

- Affected key (full base64).
- Reason for the advisory (planned rotation, emergency, post-incident
  report).
- Affected Loran versions (the version range for which the named key
  is the trust root).
- Recommended operator action (parallel-key window timeline, or
  upgrade-now-and-halt-updates).

## 7. References

- Spec §2 decision #6 — trust-pinned key constraint.
- Spec §11 — tldr-pages explicitly *not* covered by this procedure
  (tldr-pages is unsigned by upstream, so there is no rotation
  surface).
- `loran-core::signing::verify_any` — the parallel-key acceptance
  primitive.
- `loran-core::pipeline::UpdateOpts::public_keys` — the runtime
  trust-pinned set consulted by `update_pages`.
- PRD NG-09 — the publisher pipeline this section realizes (originally
  scoped as a separate, out-of-PRD project).
- `loran-pages` repo (`scripts/build-pages.sh`, `.github/workflows/{validate,publish}.yml`)
  — the producer implementation described in §2.
