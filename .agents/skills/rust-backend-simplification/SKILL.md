---
name: rust-backend-simplification
description: Simplify, clean up, or refactor OpenFlow's Rust core (apps/desktop/src-tauri) — dead code, vague errors, missing doc comments, duplicated logic, oversized modules. Use this before touching pipeline.rs, audio.rs, output.rs, stt.rs, or any threading/locking code, because several load-bearing constraints there are invisible in a diff and easy to break.
---

# Rust backend simplification

## Purpose

Make the Rust core smaller and clearer without breaking the constraints that
hold it together. The danger in this codebase is not complexity — it's that
the threading model, cancellation mechanism, and privacy invariants look like
refactoring opportunities to fresh eyes.

## Files to inspect first

- `docs/engineering/architecture-boundaries.md` — §"Constraints an agent can
  break without noticing" (read all seven before editing the named files).
- `CLAUDE.md` — threading model and pipeline invariants in brief.
- The module you're changing, top to bottom, including its tests.

## Procedure

1. Read the module's `//!` header and tests first — they encode intent.
2. Identify the cleanup type:
   - **Dead code**: verify by grepping all call sites, not by `pub` absence.
   - **Vague errors**: rewrite `AppError` messages for the HUD — sentence
     case, actionable, no module paths ("Could not open the audio input
     device", not "cpal error: …").
   - **Swallowed errors**: a `let _ =` is fine for emit calls (by design),
     wrong for anything whose failure the user must learn about.
   - **Duplication**: dedupe into the owning module; for two-site security
     logic (like `safe_id`), prefer mirrored code with cross-reference
     comments and tests on both sides, over a forced shared abstraction.
   - **Doc comments**: every IPC command and non-obvious public item gets one
     `///` line, summary-first, third person.
3. Keep diffs behavior-preserving; if behavior must change, that's a feature
   PR, not a simplification.
4. Run the full Rust gate (below) — clippy at `-D warnings` is the floor.

## Rules

- Never move audio/output/whisper work onto async executors; threads and
  `std::sync::mpsc` are deliberate (cpal `!Send`, CGEvent thread affinity,
  whisper in `spawn_blocking`).
- Never add a pipeline state write without re-checking the generation
  counter; never reorder the `fetch_add` relative to the spawn it guards.
- `OutputSystem::insert`/`capture_selection` deadlock on the main thread —
  commands offload via `spawn_blocking`.
- The HUD/changes windows are never hidden or shown — content fades.
- No `unwrap()` outside tests; poisoned-lock `expect()` is the accepted
  exception. Prefer `ok_or_else(|| AppError::…)` over new `expect`s.
- Quit paths go through `app.exit(0)` so `RunEvent::Exit` can unload the
  whisper context — `std::process::exit` reintroduces a shutdown abort.
- Don't split `pipeline.rs` or extract single-caller helpers; the documented
  seams live in `monorepo-conventions.md` §"When to split".

## Commands

```sh
cd apps/desktop/src-tauri
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
# or from the root, including the frontend build tauri-build validates:
pnpm check:rust
```

## Checklist

- [ ] All seven boundary constraints re-read if their files are touched
- [ ] Behavior unchanged (tests prove it; add one if the area had none)
- [ ] Errors that reach users are HUD-ready sentences
- [ ] New/changed public items have `///` docs
- [ ] clippy `-D warnings` + cargo test green

## Common mistakes

- "Fixing" the never-hidden HUD window or the always-broadcast settings
  event — both are documented invariants.
- Converting `std::sync::mpsc` + threads to tokio because it looks dated.
- Removing a `clone()` that exists to keep history/result data identical, or
  holding a lock across an `.await`.
- Deleting `LegacyLlmConfig`-style migration code that still serves old
  installs.

## Expected output

A behavior-preserving diff with fewer lines, clearer names, HUD-ready
errors, and doc comments — and `pnpm check:rust` green.
