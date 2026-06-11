---
name: dependency-hygiene
description: Add, update, audit, or remove dependencies in OpenFlow — npm packages, Cargo crates, cargo features, or GitHub Actions. Use this for every dependency change, including devDependencies and version bumps, because the repo is deliberately dependency-light and has supply-chain guards that silently shape what installs.
---

# Dependency hygiene

## Purpose

Every dependency on the dictation path is latency, attack surface, and a
future maintenance debt. The repo hand-rolls a resampler and a WAV encoder
on purpose. Default answer to "should we add X": no — then prove otherwise.

## Files to inspect first

- Manifests: root/`apps/desktop`/`packages/core` `package.json`,
  `apps/desktop/src-tauri/Cargo.toml` (note the feature lists).
- `pnpm-workspace.yaml` — `minimumReleaseAge: 1440`: versions younger than
  24 h refuse to install (supply-chain guard; needs pnpm ≥ 10.16, satisfied
  by the pinned `packageManager`).
- `docs/engineering/monorepo-conventions.md` §"Dependency policy".

## Procedure

**Adding:**

1. Try stdlib/hand-rolled first; for scripts, stdlib-only Node is the rule.
2. Justify in the PR: what it does, why in-house is worse, maintenance
   status, transitive weight (`pnpm why` / `cargo tree -i` after install).
3. Add to the package that uses it (not the root, unless repo tooling).
   Cargo: enable only the features used, with a one-line comment when
   non-obvious.
4. If install fails on a brand-new release, that is `minimumReleaseAge`
   working — wait it out or pin the previous version; never bypass it.
5. Telemetry-adjacent packages will fail `check:privacy` by name — that's a
   design rejection, not a false positive to allowlist.

**Updating:** read the changelog between versions; lockfile diff in its own
commit; `pnpm check:all` after. For `packageManager` bumps, same-major only
unless release notes are read end-to-end.

**Removing:** delete the dependency in the same PR that removes its last
use; `pnpm why <pkg>` / grep proves it's last.

**Auditing features:** for each Cargo feature, find a use site
(`grep -rn 'multipart' src/`); to test a suspected-unused feature, remove it
and run `cargo check --all-targets` — features are additive, so other crates'
needs are unaffected by our spec.

## Rules

- Nothing new on the record→insert path without a latency argument.
- GitHub Actions stay on major-version tags (`@v4`) like the existing ones;
  bumping one means reading its release notes.
- No Dependabot/Renovate here by decision (one maintainer, guard + lockfile
  review instead) — don't add the config "to help".
- License must be compatible with MIT distribution.

## Commands

```sh
pnpm why <package>             # who pulls it in
cd apps/desktop/src-tauri && cargo tree -i <crate> | head   # reverse deps
pnpm check:all                 # full gate after any manifest change
```

## Checklist

- [ ] Justification written (need, alternatives, weight, maintenance)
- [ ] Right manifest, minimal features, exact-enough version range
- [ ] Lockfile change isolated and reviewed
- [ ] `check:privacy` and `check:all` green
- [ ] Removal PRs leave no orphaned config behind

## Common mistakes

- Adding a 40-transitive-dep formatting library to save ten lines.
- Bypassing `minimumReleaseAge` with an override because a release is
  "obviously fine" — the guard exists precisely for that moment.
- Enabling a kitchen-sink cargo feature set ("full") instead of the two
  features used.
- Updating `packageManager` or toolchains as a drive-by in a feature PR.

## Expected output

A manifest diff with a written justification, minimal feature surface, an
isolated lockfile change, and `pnpm check:all` green.
