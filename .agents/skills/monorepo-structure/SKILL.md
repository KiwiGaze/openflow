---
name: monorepo-structure
description: Decide where code, files, scripts, and docs belong in the Velata monorepo. Use this whenever you create a new file, move code between packages, add a script or dependency, or are unsure whether something belongs in packages/core, apps/desktop/src (React), or apps/desktop/src-tauri (Rust) — even if the task doesn't mention "structure".
---

# Monorepo structure

## Purpose

Put new code where the next reader will look for it, and keep the boundary
rules intact: Rust owns the OS, React renders, `@velata/core` carries the
shared contract. Misplaced code is how backend logic leaks into the webview
and how the IPC mirror drifts.

## Files to inspect first

- `docs/engineering/monorepo-conventions.md` — layout, scripts, naming,
  when-to-split rules (authoritative).
- `docs/engineering/architecture-boundaries.md` — ownership table and the
  constraints that don't show up in a diff.
- `pnpm-workspace.yaml`, root `package.json` — what exists and how it runs.
- The directory you're adding to — match its local pattern before inventing
  one.

## Procedure

1. Classify the code you're adding:
   - Touches OS, audio, network, files, processes, or heavy compute → Rust
     (`apps/desktop/src-tauri/src/`), in the module that owns that domain.
   - Pure + shared between webviews or with tests → `packages/core/src/`.
   - Renders or wires UI state → `apps/desktop/src/app/` (settings) or the
     HUD/changes entry points.
2. If it crosses IPC, stop and load the `tauri-ipc` skill — both sides change
   together.
3. Name by domain (`suggestions.rs`, `modeio.ts`), never by shape
   (`utils.ts`, `helpers.rs`, `manager.rs`).
4. New scripts: package-level scripts keep the standard names
   (`build`/`test`/`typecheck`); repo tooling goes in `scripts/*.mjs`
   (stdlib-only Node) and gets a root `package.json` entry.
5. New docs: conventions → `docs/engineering/`, system design →
   `docs/ARCHITECTURE.md`, product/UX → `docs/design/`, user-facing →
   `README.md`.
6. Verify with the commands below.

## Rules

- One-way dependency: `apps/desktop/src` → `@velata/core`. Core never
  imports Tauri, React, or anything workspace-local.
- No new top-level directories or workspace packages without prior agreement
  in an issue — that is an architecture decision.
- Tests are colocated (`foo.test.ts` beside `foo.ts`; `mod tests` at the
  bottom of the `.rs` file). No separate test trees.
- `pnpm -r build` before `typecheck`/lint from a cold tree — the desktop app
  imports core's `dist/`.

## Commands

```sh
pnpm check          # build + ipc + privacy + lint + format + typecheck + TS tests
pnpm check:all      # the above + the cargo gate
```

## Checklist

- [ ] Code sits on the right side of the ownership table
- [ ] Names are domain words; file matches sibling conventions
- [ ] Any IPC crossing handled via the tauri-ipc skill
- [ ] Tests colocated; `pnpm check` passes

## Common mistakes

- Putting "just a little" text processing or validation in a React component
  because the Rust round-trip feels heavy — it duplicates the backend and the
  two copies drift.
- Creating `utils.ts`/`helpers.rs` dumping grounds.
- Adding a per-package ESLint/Prettier config — there is exactly one, at the
  root.
- Forgetting that `pnpm build` produces web assets only; a release build is
  `pnpm tauri build`.

## Expected output

Changed files land in the correct package with domain names, `pnpm check`
(or `check:all` when Rust changed) passes, and no boundary rule from
`architecture-boundaries.md` is violated.
