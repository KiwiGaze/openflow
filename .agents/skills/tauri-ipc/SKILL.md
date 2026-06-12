---
name: tauri-ipc
description: Add or change Velata Tauri IPC commands, events, or any serde struct/TS type that crosses the Rust↔webview boundary. Use this for every task that touches invoke/listen, #[tauri::command], packages/core/src/types.ts, COMMANDS/EVENTS, or settings/profile shapes — even a one-field change, because the contract is hand-mirrored and one-sided edits break it silently.
---

# Tauri IPC contract changes

## Purpose

Keep the hand-mirrored IPC contract exact. There is no codegen: a Rust-side
field rename silently becomes `undefined` in TypeScript unless the mirror
changes in the same diff. `pnpm check:ipc` catches name-level drift; this
skill covers the rest.

## Files to inspect first

- `docs/engineering/ipc-contract-conventions.md` — the full rules, naming
  table, and add-a-command/add-an-event checklists (authoritative).
- `apps/desktop/src-tauri/src/commands.rs` — every command handler.
- `apps/desktop/src-tauri/src/main.rs` — the `generate_handler![…]` registry.
- `packages/core/src/types.ts` — the TS mirror: types + `COMMANDS`/`EVENTS`.
- `apps/desktop/src/app/ipc.ts` — the typed wrappers (the only `invoke`/
  `listen` call sites).

## Procedure

1. Read the conventions doc's checklist for what you're doing (command vs
   event vs shape change).
2. Make the Rust change. Every IPC struct carries
   `#[serde(rename_all = "camelCase")]`; commands get a `///` summary naming
   any OS side effect.
3. Mirror in `types.ts` in the same working set: same field names, same
   optionality (`Option<T>` ↔ `T | null` — this codebase uses `null`).
4. New command → register in `generate_handler!`, add to `COMMANDS`
   (key = camelCase of the snake_case name), add a wrapper in `ipc.ts`.
5. New event → `pub const <NAME>_EVENT` in the owning module, mirror in
   `EVENTS`, add `events.on<Name>` wrapper. Never emit a string literal.
6. If the command can block on the output worker (selection capture, paste),
   offload with `spawn_blocking` exactly like `start_polish_selection` —
   running it inline on the main thread deadlocks.
7. Run the commands below; fix anything `check:ipc` reports.

## Rules

- Both sides in one diff, always. If the mirror genuinely needs no change,
  say why in the PR description.
- Naming: `get_` returns one value, `list_` returns a collection; `stop`
  finishes and processes, `cancel` discards; `save_`/`delete_` return the
  fresh list.
- Errors cross IPC as plain strings (`AppError` serialization). Never make
  the webview parse error text to branch.
- Shared wire literals (id prefixes, schema versions) are mirrored named
  constants on both sides (`CLOUD_STT_PREFIX` is the model).
- Renames are breaking wire changes: all five places in one commit, called
  out in the PR.

## Commands

```sh
pnpm check:ipc                       # drift guard (runs in pnpm check and CI)
pnpm -r build && pnpm typecheck      # the mirror must also typecheck
pnpm check:all                       # full gate when Rust changed
```

## Checklist

- [ ] Rust struct/command + TS mirror in the same diff
- [ ] `#[serde(rename_all = "camelCase")]` on every crossing type
- [ ] Registered in `generate_handler!`, mirrored in `COMMANDS`/`EVENTS`,
      wrapped in `ipc.ts`
- [ ] Doc comment on the command (side effects named)
- [ ] `spawn_blocking` if it reaches the output worker
- [ ] `pnpm check:ipc` green

## Common mistakes

- Editing `types.ts` from memory of the Rust struct instead of reading it —
  optionality mismatches survive typecheck and fail at runtime.
- Adding a command but not the `ipc.ts` wrapper, then calling `invoke`
  directly from a component (wrong by construction).
- Emitting an event with a string literal — `check:ipc` fails the build.
- Changing what an existing command returns without checking every wrapper
  call site.

## Expected output

A diff where the Rust side, the `types.ts` mirror, and the `ipc.ts` wrappers
move together; `pnpm check:ipc` and `pnpm check:all` pass.
