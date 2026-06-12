---
name: react-ui-simplification
description: Simplify or clean up Velata's React webviews (apps/desktop/src) — oversized tab components, duplicated constants, loose casts, hook hygiene, IPC wiring. Use this for any change to App.tsx, Hud.tsx, Changes.tsx, tabs/, components/, or hooks.ts, and especially before extracting components or "improving" state management.
---

# React UI simplification

## Purpose

Keep the webviews thin. They render Rust-owned state and dispatch IPC; every
simplification should move them closer to that, never toward a second
frontend brain (state libraries, client-side business logic, local copies of
backend values).

## Files to inspect first

- `docs/engineering/architecture-boundaries.md` — what React must not own.
- `docs/engineering/monorepo-conventions.md` §"When to split" — the seam
  policy. The two previously sanctioned seams are taken: `ModeEditor` and
  `LlmProfileEditor` in `components/`.
- `apps/desktop/src/app/hooks.ts` — the established hook patterns
  (optimistic save + rollback, bootstrap-once + subscribe).
- `apps/desktop/src/app/ipc.ts` — the only place `invoke`/`listen` appear.
- `packages/core/src/` — check here before defining any constant or
  validation in a component.

## Procedure

1. Before writing UI logic, ask: does Rust or `@velata/core` already own
   this value or rule? Import it; never re-declare presets, versions, wire
   prefixes, or validation.
2. For state: settings flow through `useSettings` (optimistic update,
   rollback on save error). New backend state gets the same shape — one
   bootstrap `invoke` on mount, then event subscription; no per-component
   fetch storms, no polling where an event exists.
3. Every `listen()` goes through the `events.on*` wrappers and cleans up via
   the returned unlisten in the effect cleanup (Strict Mode double-mount
   duplicates handlers otherwise).
4. Replace `as` casts from user/DOM input with typed sources (a union-typed
   constant, a type guard) — a cast that papers over a design gap hides
   real breakage when ids change.
5. Extract a component only at a documented seam or when a block has its own
   state and a clear name; extract a hook only when two components share the
   stateful logic.
6. Run the TS gate (below).

## Rules

- No `fetch`/XHR/WebSocket in webview code — `check:privacy` fails the
  build; every byte goes through Rust where the consent gates live.
- No text processing, prompt building, or consent logic in React — that is
  backend behavior the webview would duplicate.
- Strict TS holds: no `any`, explicit return types on exported functions,
  module-level constants for render-invariant values.
- Presentation logic (tab selection, fade timing, form drafts) belongs here —
  don't push it into Rust either.
- UI strings: sentence case, plain English, actionable.

## Commands

```sh
pnpm -r build && pnpm lint && pnpm typecheck && pnpm -r test
pnpm check        # the same plus ipc/privacy/format gates
```

## Checklist

- [ ] No value re-declared that `@velata/core` exports
- [ ] Effects clean up listeners; no per-render `invoke`
- [ ] No new `as` casts from untyped input
- [ ] Extractions only at documented seams or genuinely shared logic
- [ ] `pnpm check` green; screenshots if the UI visibly changed

## Common mistakes

- Hardcoding `'cloud:'`, profile versions, or Ollama defaults in a component
  — they're exported from `@velata/core` precisely so they can't drift.
- Adding a state-management library; `useState` + IPC events is the whole
  model and it fits in one file.
- Splitting a 400-line tab into five fragments that share mutable state —
  the seams doc names the splits that actually reduce complexity.
- Swallowing IPC errors instead of surfacing them like `useSettings` does
  (`saveError` + rollback).

## Expected output

A thinner component tree with shared values imported from core, disciplined
hooks, no new casts, and `pnpm check` green — plus before/after screenshots
for any visible change.
