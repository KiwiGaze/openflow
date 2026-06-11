# Monorepo conventions

How this repository is laid out and why, plus the rules that keep it boring.
Boundaries between Rust, React, and the shared package live in
[architecture-boundaries.md](architecture-boundaries.md); the IPC rules in
[ipc-contract-conventions.md](ipc-contract-conventions.md); what reviewers
check in [review-checklist.md](review-checklist.md).

## Root layout

```
apps/desktop/          The one application: React webviews + src-tauri Rust core
packages/core/         @openflow/core — shared TS types (IPC mirror) + pure utilities
docs/                  ARCHITECTURE, DEVELOPMENT, PRD + design/ + engineering/
scripts/               Repo tooling: check-ipc.mjs, check-privacy.mjs, generate-icon.mjs, release.sh
.agents/skills/        Repo-local skills for AI coding agents (procedures + guardrails)
.github/               CI (ci.yml), release (release.yml), PR/issue templates
```

Top-level dotfiles each do one job: `tsconfig.base.json` (shared compiler
options), `eslint.config.mjs` (one flat config for the whole repo — packages
do not carry their own), `.prettierrc.json`, `.editorconfig`, `.nvmrc`
(the Node version CI uses; `engines` in package.json is the older floor),
`.coderabbit.yaml` (automated review tuned to these same conventions).

New code goes in an existing home. Creating a new top-level directory or a
new workspace package is an architecture decision — propose it in an issue
first.

## Scripts

Rules:

- Every package exposes the same base names — `build`, `test`, `typecheck` —
  so `pnpm -r <name>` works with no per-package knowledge. Variants are
  colon-namespaced (`build:vite`, `check:ipc`).
- The root owns linting and formatting (`pnpm lint`, `pnpm format:check`)
  because there is one ESLint/Prettier config for the repo.
- `pnpm check` runs exactly what the CI `web` job runs, in the same order;
  `pnpm check:rust` mirrors the `rust` job; `pnpm check:all` is both. If you
  change one side, change the other in the same commit — the pair drifting is
  a bug.
- Build before typecheck: `@openflow/desktop` imports `@openflow/core` from
  its `dist/`, so from a cold clone run `pnpm -r build` (or just `pnpm check`)
  before `typecheck`, type-aware lint, or tests.
- `pnpm dev` is the only dev entry point (`tauri dev`: Vite + Rust with hot
  reload). `dev:vite` inside apps/desktop exists for `tauri.conf.json`'s
  `beforeDevCommand`; don't call it directly.
- Note `pnpm build` produces web assets only; a release artifact is
  `pnpm tauri build`.

## Tests, mocks, fixtures

- TS tests are vitest, colocated next to the source as `<module>.test.ts`.
  Rust tests are `#[cfg(test)] mod tests` at the bottom of the module they
  test. Neither side has a separate `tests/` tree — keep it that way until a
  test needs cross-module fixtures.
- Logic changes require tests (text processing, settings migration, IPC
  shapes, validation). GUI behavior, TCC permission flows, and real paste are
  manual-only — the checklist lives in `docs/DEVELOPMENT.md`.
- Filesystem-touching Rust tests use `tempfile`; no test may read or write
  real app-data paths.
- There are no mock frameworks. Pure functions take values; the few stateful
  managers take a directory path, which tests point at a tempdir. Prefer
  extending that pattern over introducing mocking machinery.
- The real-model STT integration test is `#[ignore]` and opt-in:
  `OPENFLOW_TEST_MODEL=/path/to/ggml-tiny.en.bin cargo test -- --ignored`.

## Logging and errors

- Rust: `log::warn!`/`log::info!` via `tauri_plugin_log` (terminal +
  `~/Library/Logs/app.openflow.desktop/`). Log messages may name modules and
  errors freely — they are for triage.
- Never log transcript text, selection contents, audio data, or API keys.
  Provider response bodies on the audio path are logged status-only
  (see `cloud_stt.rs`).
- User-facing failures become `AppError` (in `error.rs`) and surface as HUD
  notices via `AppError::user_message()` — sentence case, no module paths, no
  Rust type names, tell the user what to do next. Dictation output is never
  silently dropped: worst case the text lands on the clipboard and the HUD
  says so.
- The webviews don't log. If a webview needs to report something, that is a
  sign the logic belongs in Rust.

## Environment variables

The app reads none at runtime — configuration is `settings.json` plus profile
files, and that is a privacy feature: behavior cannot be changed invisibly
from the environment. The only env var in the repo is `OPENFLOW_TEST_MODEL`
(opt-in STT integration test). Adding a runtime env var requires updating
PRIVACY.md and this section — treat it as a design change.

## Docs placement

- `README.md` — users: what it does, install, first run.
- `CONTRIBUTING.md` — contributors: ground rules, PR workflow, conventions.
- `CLAUDE.md` (`AGENTS.md` symlinks to it) — AI agents and quick-start
  contributors: commands, architecture summary, invariants. Keep it short;
  link instead of inlining.
- `docs/ARCHITECTURE.md` — how the system works and why (tradeoffs).
- `docs/DEVELOPMENT.md` — environment setup, daily loop, manual test
  checklist, release flow.
- `docs/engineering/` — conventions (this directory) and research notes.
- `docs/design/` — product/UX design package; specs reference these as
  `(07 §4)` etc.
- Package READMEs (`packages/core`, `apps/desktop`) — one screen each:
  what the package is, what may depend on it, where its rules live.

When code and a doc disagree, the code is right; fix the doc in the same PR
that exposed the drift.

## Naming

- Domain words over generic words: `resample`, `pipeline`, `suggestions` —
  never `utils`, `helpers`, `manager2`, `data`, `item`, `temp`. If the only
  honest name is `util.rs`, the code hasn't found its home yet.
- Rust: modules and functions `snake_case`; getters without a `get_` prefix
  (`pipeline.state()`, not `get_state()`); conversions follow cost —
  `as_` borrows, `to_` allocates, `into_` consumes.
- IPC command names are a wire contract and follow their own rules — see
  [ipc-contract-conventions.md](ipc-contract-conventions.md).
- TS: `camelCase` values, `PascalCase` types/components, `SCREAMING_SNAKE`
  for shared constants mirrored with Rust (`CLOUD_STT_PREFIX`).
- React hooks start with `use` if and only if they call hooks.

## Comments

- Comments explain **why**: invariants, OS quirks, tradeoffs, failure modes,
  ordering requirements. The code says what it does.
- Public Rust items that cross the IPC boundary or have non-obvious semantics
  get a `///` summary — one declarative sentence, third person ("Returns …",
  "Deletes …"). Module headers are `//!` and state what the module owns.
- Shared TS types in `packages/core` carry doc comments because they are the
  written half of the IPC contract.
- Spec references like `(08 §3)` point into `docs/design/`; keep them — they
  are the paper trail for why a behavior exists.
- Delete comments that restate the line below them. A wrong comment is worse
  than no comment; if you change behavior, the comment changes in the same
  diff.

## When to split a file, module, hook, or service

Split on **responsibility**, not on line count. The trigger: the file answers
two unrelated questions, or you must scroll past one concern to edit another.

- A Rust module splits when it owns two domains. Current judged-and-deferred
  cases: `pipeline.rs` (1.2k lines) mixes the state machine with dictation
  and selection processing, but both halves share the generation counter and
  session state — split only when adding a new job type forces the issue
  (the seam: `resolve_dictation`/`finish_*` into a sibling module that takes
  `&Pipeline`). `commands.rs` stays one file on purpose: a single place to
  read the whole IPC surface.
- A React component splits when a self-contained block has its own state.
  Known seams, fine to take when touching those files: the mode editor card
  in `ModesTab.tsx` (~lines 270+) and the LLM profile editor card in
  `ModelsTab.tsx` (~lines 317+).
- Extract a hook when two components need the same stateful logic, or when
  naming one effect makes it understandable. Not before.
- Never split just to make files smaller — ten 100-line files with shared
  mutable state are harder to read than one honest 500-line file.

## Dependency policy

Keep the dictation critical path dependency-light. Before adding a dependency:
the stdlib/hand-rolled option was considered (this repo resamples audio and
encodes WAV by hand on purpose); the package is established and maintained;
`pnpm install` respects `minimumReleaseAge` (versions younger than 24 h are
refused — don't bypass it). Remove a dependency in the same PR that obsoletes
it.
