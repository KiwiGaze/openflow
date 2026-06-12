# IPC contract conventions

The Rust core and the React webviews speak over Tauri IPC with a
**hand-mirrored** contract — no codegen. This is a deliberate tradeoff
(evaluated against tauri-specta and ts-rs in
[research/monorepo-best-practices.md](research/monorepo-best-practices.md) §3):
the surface is small enough that discipline plus a drift check beats managing
RC codegen crates. The discipline is this document; the drift check is
`pnpm check:ipc` (`scripts/check-ipc.mjs`), which runs in `pnpm check` and CI.

## Where the contract lives

| Piece                         | Rust (authoritative for behavior)                                                                                                                                                   | TypeScript mirror                              |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| Command handlers              | `#[tauri::command]` fns in `src-tauri/src/commands.rs`                                                                                                                              | `COMMANDS` map in `packages/core/src/types.ts` |
| Command registry              | `generate_handler![…]` in `src-tauri/src/main.rs`                                                                                                                                   | —                                              |
| Event names                   | `pub const *_EVENT` in the module that owns the event                                                                                                                               | `EVENTS` map in `types.ts`                     |
| Payload/return shapes         | serde structs in `settings.rs`, `pipeline.rs`, `models.rs`, `profiles.rs`, `stt_profiles.rs`, `stats.rs`, `suggestions.rs`, `history.rs`, `permissions.rs`, `llm.rs`, `commands.rs` | interfaces/unions in `types.ts`                |
| Typed call/subscribe wrappers | —                                                                                                                                                                                   | `apps/desktop/src/app/ipc.ts`                  |
| Shared wire literals          | e.g. `CLOUD_STT_PREFIX` in `stt_profiles.rs`                                                                                                                                        | same name in `types.ts`                        |

The webviews never call `invoke()` or `listen()` directly — only through the
`ipc` / `events` wrappers in `ipc.ts`, which take their names from
`@openflow/core`. That keeps the entire contract reviewable in three files.

## Naming rules

- Command names: `snake_case`, verb-first.
  - `get_<thing>` returns one value; `list_<things>` returns a collection
    (`get_history` predates the rule and stays — wire renames are churn).
  - `start_/stop_/cancel_` for pipeline lifecycle. `stop` means "finish and
    process"; `cancel` means "discard".
  - `save_` upserts and returns the fresh list; `delete_` removes and returns
    the fresh list. `reveal_` opens a folder in Finder; `open_` opens a
    System Settings pane; `test_` performs a connectivity round-trip.
- `COMMANDS` keys are the camelCase form of the snake_case value —
  `getSettings: 'get_settings'` (checked mechanically).
- Event names: kebab-case strings, defined once as a Rust
  `pub const <NAME>_EVENT: &str` in the module that owns the event, mirrored
  in `EVENTS`. **Never emit a string literal** — `check:ipc` fails the build
  on `.emit("…")`.
- Wire values that both sides parse (id prefixes, schema versions) get a
  named constant on each side, with a doc comment naming its mirror
  (`CLOUD_STT_PREFIX`, `LLM_PROFILE_VERSION`, `STT_PROFILE_VERSION`).

## Shape rules

- Every struct/enum that crosses IPC carries
  `#[serde(rename_all = "camelCase")]`. Without it, a snake_case field
  arrives in TS as `undefined` — silently. Enum variants serialize camelCase
  too (`'polishSelection'`).
- The TS mirror uses the same field names, the same optionality (`Option<T>`
  ↔ `T | null` — this codebase uses `null`, not `undefined`, for absent
  values), and mirrors doc comments where the meaning is non-obvious.
- Commands that can fail return `AppResult<T>`; `AppError` serializes to a
  plain message string (see `error.rs`). The webview treats a rejected
  `invoke` as a display string — it must not parse error text to branch.
  If the UI ever needs to branch on error kind, extend `AppError`'s
  serialization to `{ kind, message }` on both sides in one PR; don't match
  on substrings.
- Fire-and-forget operations return `()` and report progress/failure via an
  event (`download_model` → `model-download`).

## Adding a command (checklist)

1. Write the `#[tauri::command]` fn in `commands.rs` with a `///` summary
   (what it does, and any constraint a caller must know).
2. Register it in `generate_handler![…]` in `main.rs`.
3. Add the entry to `COMMANDS` in `packages/core/src/types.ts`, key =
   camelCase of the name.
4. Mirror any new payload/return types in `types.ts`.
5. Add a typed wrapper to `apps/desktop/src/app/ipc.ts`.
6. If the command blocks on the output worker (selection, paste), offload via
   `spawn_blocking` — see `start_polish_selection` and
   [architecture-boundaries.md](architecture-boundaries.md) §constraint 2.
7. Run `pnpm check:ipc` (steps 1–3 are mechanically verified; 4–5 are caught
   by `pnpm typecheck` only if something consumes them — review covers the
   rest).

## Adding an event (checklist)

1. `pub const <NAME>_EVENT: &str = "<kebab-name>";` in the owning Rust module,
   with a doc comment stating payload type and when it fires.
2. Emit with the constant, payload `#[serde(rename_all = "camelCase")]`.
3. Mirror in `EVENTS` + payload type in `types.ts`.
4. Add an `events.on<Name>` wrapper in `ipc.ts`; subscribers clean up via the
   returned unlisten (see `subscribe()` helper).
5. Prefer `emit` (broadcast) only when more than one window cares; for a
   single-window event use `emit_to(label, …)`.

## What `check:ipc` verifies (and what it can't)

Verified mechanically: command set ↔ `COMMANDS` (both directions), command
set ↔ `generate_handler!`, event constants ↔ `EVENTS`, no literal event
names at emit sites, `COMMANDS` key/value casing, mirrored shared literals.

Not verifiable mechanically (review carries these): field-level shape
equality between a serde struct and its TS interface, optionality mismatches,
enum variant sets, and whether a changed Rust struct's mirror was updated.
That is why the PR template asks "IPC contract changes mirrored?" and why
both files must appear in the same diff.

## Review rules for IPC diffs

- A diff touching a serde struct that crosses IPC **must** touch
  `packages/core/src/types.ts` in the same PR (or state why no mirror change
  is needed).
- A diff adding `invoke`/`listen` outside `ipc.ts` is wrong by construction.
- New commands with side effects on the OS (paste, files, permissions) need a
  doc comment naming the side effect — the TS caller can't see it otherwise.
- Renaming a command or event is a breaking wire change: update all five
  places in one commit and call it out in the PR description.
