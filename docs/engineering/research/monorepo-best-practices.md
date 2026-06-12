# Monorepo best practices — research notes

Compiled 2026-06-11 from official documentation and engineering writeups, then
filtered against this repo's reality (2 packages, 1 app, macOS-only, privacy
as the product). Each section ends with what Velata adopted and what it
deliberately skipped — "skip" here is a decision, not an omission.

Companion docs that turn these notes into rules:
[monorepo-conventions](../monorepo-conventions.md) ·
[architecture-boundaries](../architecture-boundaries.md) ·
[ipc-contract-conventions](../ipc-contract-conventions.md) ·
[review-checklist](../review-checklist.md)

## 1. pnpm workspace and scripts

- Reference internal packages with `workspace:*` so resolution can never fall
  back to the registry ([pnpm workspaces](https://pnpm.io/workspaces)).
  Already the case (`@velata/core: workspace:*`).
- Use the same base script names (`build`, `test`, `typecheck`) in every
  package so `pnpm -r <name>` works without per-package knowledge
  ([pnpm scripts](https://pnpm.io/scripts)). Already the case.
- Provide one root command that runs exactly what CI runs — drift between
  local commands and CI steps causes "passes locally, fails in CI"
  ([npm-scripts naming](https://literat.dev/blog/2024-12-14/mastering-npm-scripts-best-practices-in-sustainable-naming-and-organizing-of-your-scripts/)).
- `pnpm install --frozen-lockfile` in CI so lockfile drift fails the build.
  Already the case.
- pnpm's `minimumReleaseAge` refuses to install package versions younger than
  the configured age — a supply-chain guard against compromised releases
  ([pnpm settings](https://pnpm.io/settings)). It was configured in
  `pnpm-workspace.yaml` but silently inactive: the feature needs pnpm ≥ 10.16
  and `packageManager` pinned 10.9.0.

**Adopted:** root `check` / `check:rust` / `check:all` scripts that mirror the
two CI jobs command-for-command; `check:ipc` and `check:privacy` guard scripts
wired into both `check` and CI; `packageManager` bumped to pnpm 10.34.2 to
activate `minimumReleaseAge`.

**Skipped:** Turborepo/Nx (task-graph caching pays off at many packages; at
two packages the orchestration overhead exceeds the benefit — see
[Nx on pnpm monorepos](https://nx.dev/blog/setup-a-monorepo-with-pnpm-workspaces-and-speed-it-up-with-nx)),
pnpm catalogs (version centralization for ~4+ packages sharing dependencies;
we have two), `--no-bail` test runs and hidden `.script` names (machinery
without a present problem).

## 2. Shared TypeScript package shape

`@velata/core` is a **built** internal package: `tsc` emits `dist/`, and
`apps/desktop` consumes the compiled output. The known cost is the
build-before-typecheck trap — from a cold clone, `pnpm typecheck` and
type-aware lint fail until `pnpm -r build` has produced `dist/`.

The alternative is the "internal package" pattern: point `exports` at
`src/index.ts` and let Vite/vitest/tsc consume the source directly, deleting
the build step entirely
([Turborepo: you might not need project references](https://turborepo.dev/blog/you-might-not-need-typescript-project-references)).

**Adopted:** keep the built shape for now; mitigate the trap mechanically —
`pnpm check` builds first, CI builds first, and the CI step carries a comment
explaining the ordering.

**Skipped (revisit when the trap bites again):** the source-export migration.
It is a small diff with a wide blast radius (lint/typecheck/vitest/tauri-build
all re-resolve the package), which made it the wrong passenger on a
conventions PR. The migration, when wanted: set `main`/`types`/`exports` to
`./src/index.ts`, delete `tsconfig.build.json` and the `build` script, drop
the core pre-build from `dev:vite`/`build:vite`, keep `typecheck` as-is, and
run `pnpm check:all` to validate. Do not publish a source-export package to
npm without switching `exports` back to `dist/` via `publishConfig`.

## 3. Tauri 2 IPC

- Commands for request/response, events for backend-initiated push, channels
  for high-throughput streams
  ([Tauri IPC](https://v2.tauri.app/concept/inter-process-communication/)).
  Velata follows this; the audio-level feed is the one candidate where a
  channel would beat repeated events — not worth churn at one emit per frame.
- All commands registered in a single `generate_handler!` — multiple
  `invoke_handler` calls silently discard earlier ones
  ([calling Rust](https://v2.tauri.app/develop/calling-rust/)). Already the
  case; `check-ipc.mjs` now verifies the registry matches the definitions.
- `#[serde(rename_all = "camelCase")]` on every IPC-crossing struct as a
  blanket rule — a snake_case field on the wire arrives as `undefined` in TS
  with no error. Already the case; now written down as a rule.
- Codegen options: **tauri-specta** is still RC and requires exact-version
  pinning of three crates together
  ([specta-rs/tauri-specta](https://github.com/specta-rs/tauri-specta));
  **ts-rs** exports types but knows nothing about `invoke`/events, so the
  wrappers stay hand-written anyway. For a ~41-command surface, a disciplined
  hand-mirror plus a drift check costs less than managing RC codegen.
- `emit_to(label, …)` targets one window instead of broadcasting
  ([calling the frontend](https://v2.tauri.app/develop/calling-frontend/)).

**Adopted:** hand-mirrored contract kept and now enforced by
`scripts/check-ipc.mjs` (commands ↔ `COMMANDS`, registry, events ↔ `EVENTS`,
no string-literal emits, camelCase key check); shared wire literals promoted
to mirrored constants (`CLOUD_STT_PREFIX`, profile versions).

**Skipped:** tauri-specta/ts-rs adoption (above); switching existing
broadcasts to `emit_to` (both webviews are tiny and the HUD ignores what it
doesn't subscribe to — converting is churn with no observable win today; use
`emit_to` for future window-specific events).

## 4. React + Vite inside Tauri

- The webview is a rendering layer: business logic lives in Rust, components
  render state and dispatch `invoke()`
  ([Tauri process model](https://v2.tauri.app/concept/process-model/)).
- One typed wrapper module around `invoke`/`listen` rather than scattered
  string literals — Velata's `ipc.ts` already is this pattern.
- Every `listen()` subscription returns its unlisten from `useEffect` cleanup,
  or Strict-Mode double-mounting duplicates handlers.
- Multiple windows = multiple Vite HTML entries (`index.html`, `hud.html`,
  `changes.html`) so neither window loads the other's bundle. Already the
  case.
- Bootstrap Rust-owned state with one `invoke` on mount, then subscribe to
  change events — per-component fetching is an API storm.

**Adopted:** written into
[architecture-boundaries](../architecture-boundaries.md) as review rules; the
audit confirmed no backend logic is currently duplicated in React.

**Skipped:** Tauri's isolation pattern (sandboxed iframe between frontend and
IPC) — it defends against third-party scripts in the webview, and Velata
loads none.

## 5. Local-first privacy engineering

- State privacy invariants as rejection rules, not prose, in the agent/contributor
  docs ([OpenSSF AI-assistant guide](https://best.openssf.org/Security-Focused-Guide-for-AI-Code-Assistant-Instructions.html)).
  CLAUDE.md already does this ("PRs that add telemetry … will not be merged").
- Enumerate permitted outbound endpoints so any new one is visibly a policy
  change. PRIVACY.md already does this.
- Add mechanical tripwires for the accidents that are cheap to catch: network
  crates outside audited modules, webview-originated network I/O, telemetry
  SDKs in manifests.
- Consent gates: appear at first activation, name the provider, say what data
  leaves, require explicit confirmation. Velata's STT consent gate already
  does this, including revoking consent when the endpoint changes.

**Adopted:** `scripts/check-privacy.mjs` (the three tripwires above), run in
`pnpm check` and CI.

**Skipped (candidates for later, recorded as future improvements):** storing
BYO API keys in the macOS Keychain via the `keyring` crate instead of 0600
profile files — a real upgrade but a feature-level change with a migration;
an `AudioBuffer` newtype that doesn't implement `Serialize` so audio
persistence becomes a compile error — elegant, but touches the whole audio
path for an invariant that is currently easy to review by hand.

## 6. Mixed Rust + TypeScript CI

- Cache both sides: `setup-node` with `cache: pnpm`, and `Swatinem/rust-cache`
  keyed after the toolchain step
  ([Tauri CI guide](https://v2.tauri.app/distribute/pipelines/github/),
  [rust-cache](https://github.com/Swatinem/rust-cache)). Already the case,
  in the correct order.
- Path filtering (`dorny/paths-filter`) can skip the macOS Rust job on
  TS-only changes, but skipped required checks block merges unless an
  `if: always()` aggregator job translates skipped → success
  ([discussion](https://github.com/orgs/community/discussions/26251)).
- Keep CI steps and local commands identical; the workflow should read as the
  same list a contributor runs.

**Adopted:** CI now runs `check:ipc` + `check:privacy`; `pnpm check:all`
mirrors both jobs end-to-end locally.

**Skipped:** path filtering + aggregator (two extra moving parts; the Rust
job is ~minutes with a warm cache and the repo has one active maintainer —
revisit when CI time actually hurts); pinning `dtolnay/rust-toolchain` to a
version (CLAUDE.md documents `rust-version = 1.82` as the floor; tracking
stable is fine at this scale). `release.yml` deliberately runs no checks —
tags are cut from `main`, which only advances through CI-green PRs.

## 7. Readability and maintainability

- Split modules by responsibility, not line count — the trigger is "this file
  answers two different questions", not a number
  ([Rust book ch. 7](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)).
  Concrete split seams for the two large files that mix concerns are recorded
  in [monorepo-conventions](../monorepo-conventions.md) §"When to split".
- Rust naming: getters without `get_` (C-GETTER), conversions signal cost via
  `as_`/`to_`/`into_` (C-CONV), no `util`/`helper`/`manager` modules
  ([Rust API guidelines](https://rust-lang.github.io/api-guidelines/naming.html)).
  This repo's existing `get_*` IPC command names are kept — wire names are a
  public contract and renames are churn — but the read-single vs
  list-collection distinction is now documented.
- Doc comments: summary-first, third-person ("Returns …"), don't restate the
  signature; `//!` module headers state what the module owns
  ([rustdoc book](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)).
- Errors: `thiserror` for the typed app error, message text written for the
  HUD (sentence case, no module paths, actionable); add context only at
  module-boundary crossings so messages don't nest the same words twice.
  Velata's `AppError` + `user_message()` already implement this shape.
- React: extract a hook when logic is shared by two components or when
  naming the effect clarifies it; never `use`-prefix a non-hook
  ([react.dev custom hooks](https://react.dev/learn/reusing-logic-with-custom-hooks)).

**Adopted:** doc comments on all 41 IPC commands and key public items; vague
or duplicated literals replaced with named constants; the naming/comment
rules codified in [monorepo-conventions](../monorepo-conventions.md).

**Skipped:** renaming wire-visible IPC commands for naming purity
(`get_history` → `list_history` would touch four layers for zero behavior);
splitting `pipeline.rs` (1,199 lines) — its halves share the generation
counter and session state, so a split today adds indirection; the seam is
documented for when the file grows again.

## 8. Agent-friendly repository docs

- Keep CLAUDE.md short enough that every line earns its place; bloated files
  get ignored ([Claude Code best practices](https://code.claude.com/docs/en/best-practices)).
- Front-load the invariants an agent must never violate; attention is
  strongest at the edges of the file
  ([HumanLayer on CLAUDE.md](https://www.humanlayer.dev/blog/writing-a-good-claude-md)).
- Put workflow recipes in on-demand skills (SKILL.md with `name`/`description`
  frontmatter) instead of always-loaded context
  ([agent skills docs](https://code.claude.com/docs/en/skills)).
- `AGENTS.md` is the cross-tool standard ([agents.md](https://agents.md/));
  Velata already symlinks it to CLAUDE.md.
- Don't restate linter-enforced style rules in agent docs; the linter is the
  enforcement.

**Adopted:** nine repo-local skills under `.agents/skills/` (structure, IPC,
Rust/React simplification, readability, privacy, CI gate, dependency hygiene,
code review); CLAUDE.md updated to point at the conventions docs and the
`check:*` commands instead of growing inline.

**Skipped:** post-edit lint hooks in `.claude/settings.json` (a per-user
choice, not a repo convention); CODEOWNERS and Dependabot/Renovate (one
active maintainer; `minimumReleaseAge` plus lockfile review covers the
supply-chain angle for now — revisit with more contributors).
