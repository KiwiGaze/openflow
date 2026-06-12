---
name: ci-quality-gate
description: Run, fix, or modify Velata's quality gates — the pnpm check/check:rust/check:all commands, the CI workflow, or any failing lint/typecheck/test/clippy step. Use this before claiming any change is done, when CI fails, or when a check command errors locally, including "works on my machine" mysteries like typecheck failing on a fresh clone.
---

# CI quality gate

## Purpose

One list of checks, three places it appears — root scripts, CI workflow,
docs — kept identical so "passes locally" means "passes in CI". This skill
is how you run the gate, diagnose the common failures, and change the gate
without splitting that source of truth.

## Files to inspect first

- Root `package.json` — the `check*` scripts (what actually runs).
- `.github/workflows/ci.yml` — the same list as two jobs (`web` on ubuntu,
  `rust` on macos-14).
- `scripts/check-ipc.mjs`, `scripts/check-privacy.mjs` — what the guard
  steps enforce and their error messages.
- `docs/DEVELOPMENT.md` §Checks — the human-readable expansion.

## Procedure

1. Run `pnpm check:all` from the repo root. Order matters and is encoded in
   the script: build → ipc → privacy → lint → format → typecheck → TS tests,
   then frontend build → cargo fmt → clippy → cargo test.
2. On failure, fix at the failing layer and re-run just that command while
   iterating; finish with one clean `pnpm check:all`.
3. If you change what the gate runs: update root `package.json`, `ci.yml`,
   `docs/DEVELOPMENT.md`, and CLAUDE.md's Commands block in the same commit.
   A gate that differs between those four places is a bug.
4. Never weaken a gate to get green (skipping a test, allowing a lint,
   loosening clippy) without saying so loudly in the PR.

## Rules

- TS checks need `pnpm -r build` first: `@velata/desktop` imports
  `@velata/core` from `dist/`. Hundreds of `no-unsafe-*` lint errors on a
  fresh tree mean exactly this.
- The Rust job builds the frontend first because tauri-build validates
  `frontendDist` — `pnpm check:rust` mirrors that; don't "optimize" it away.
- CI installs with `--frozen-lockfile`; if it fails there, your
  `package.json`/lockfile drifted — fix the lockfile, don't re-resolve in CI.
- `cargo` missing in non-interactive shells → use `$HOME/.cargo/bin/cargo`.
- Tests must pass as a suite; a flaky test gets fixed or removed, not
  retried until green.

## Commands

```sh
pnpm check:all          # everything CI runs, in CI order
pnpm check              # TS half          pnpm check:rust   # Rust half
pnpm lint:fix           # auto-fixable lint
pnpm format             # prettier write (format:check is the gate)
# single tests:
pnpm --filter @velata/core test hotkey
cd apps/desktop/src-tauri && cargo test resample::
```

## Checklist

- [ ] `pnpm check:all` exits 0 locally before any "done" claim or PR
- [ ] Gate changes mirrored in package.json + ci.yml + DEVELOPMENT.md +
      CLAUDE.md
- [ ] No gate weakened silently
- [ ] New logic carries tests the gate will run

## Common mistakes

- Running `pnpm typecheck` on a fresh clone and "fixing" phantom type errors
  that a `pnpm -r build` would have dissolved.
- Adding a CI step without the matching root script (or vice versa) — the
  next contributor can't reproduce CI locally.
- Treating `release.yml` as a check — it deliberately runs none; merges are
  gated by `ci.yml`, tags are cut from green `main`.
- Letting prettier and eslint fight: `format:check` runs after `lint`, and
  `eslint-config-prettier` already disables conflicting rules — don't add
  formatting lint rules.

## Expected output

`pnpm check:all` exiting 0, and — if the gate itself changed — all four
gate-describing files moving in lockstep in one commit.
