# Architecture boundaries

Who owns what, and the constraints that are easy to break because they are
invisible in a diff. `docs/ARCHITECTURE.md` explains how the system works;
this file is the rulebook for changing it.

## Ownership

**Rust (`apps/desktop/src-tauri/src/`) owns everything that touches the OS or
heavy compute:** audio capture (`audio.rs`), resampling (`resample.rs`),
whisper STT (`stt.rs`) and the opt-in cloud STT upload (`cloud_stt.rs`), LLM
HTTP (`llm.rs`), model downloads (`models.rs`), clipboard/paste (`output.rs`),
global hotkeys (`shortcuts.rs`), permissions (`permissions.rs`), tray
(`tray.rs`), windows (`hud.rs`, `changes.rs`), frontmost-app identity
(`apps.rs`), persistence (`settings.rs`, `profiles.rs`, `stt_profiles.rs`,
`history.rs`), the pipeline state machine (`pipeline.rs`), session stats
(`stats.rs`, `suggestions.rs`), text cleanup (`text.rs`), prompts
(`modes.rs`), and the IPC surface (`commands.rs`, wired in `main.rs`,
shared state in `state.rs`, errors in `error.rs`).

**React (`apps/desktop/src/`) is pure UI over IPC.** It renders state, calls
`ipc.*` wrappers, and subscribes to events. It must not duplicate backend
logic: no text processing, no consent decisions, no state machines that Rust
also implements, no direct network I/O (`check-privacy.mjs` fails the build
on `fetch`/XHR/WebSocket in the webview). Pure _presentation_ logic (which
tab is open, HUD fade timing, optimistic form state) belongs here.

**`packages/core` (`@openflow/core`) owns the shared TS surface:** the IPC
mirror (`types.ts`) and pure, dependency-free utilities (hotkey parsing,
formatting, validation, diff, CSV/JSON import-export, presets, languages).
The litmus test for putting code in core: it is pure, both webviews (or a
webview and a test) need it, and it imports nothing but other core modules.
Core must never import from `@tauri-apps/*` or React.

Dependency direction is one-way: `apps/desktop/src` → `@openflow/core`.
Nothing imports from `apps/desktop`, and core imports nothing workspace-local.

## The IPC boundary

Hand-mirrored and guarded by `pnpm check:ipc`. Full rules in
[ipc-contract-conventions.md](ipc-contract-conventions.md). The short
version: Rust serde structs ↔ `packages/core/src/types.ts`, camelCase on the
wire, both sides change in the same PR.

## Threading model (deliberate — do not "fix")

- `cpal::Stream` is `!Send`: it lives on the dedicated `openflow-audio`
  thread.
- `Enigo` + `arboard` live on the dedicated `openflow-output` thread (CGEvent
  thread affinity; also serializes paste/copy sequences).
- Whisper runs in `spawn_blocking` behind a `Mutex`; the loaded context is
  reused because loading maps hundreds of MB.
- Cross-thread communication is `std::sync::mpsc` with reply channels. Shared
  state is only the settings `RwLock`, the pipeline `Mutex`es, and the
  audio-level atomic. Do not move these onto async executors.

## Constraints an agent can break without noticing

Each of these is enforced only by code structure and comments — there is no
type-level guard. Check this list before touching the named files.

1. **Generation counter (`pipeline.rs`).** Cancellation works because every
   async stage captures the generation at spawn and re-checks it before
   publishing state; `cancel()` just bumps the counter. Never add a
   `set_state` call without a generation check, and never move the
   `fetch_add` after the spawn it guards.
2. **Output-thread deadlock (`output.rs`, `commands.rs`).**
   `OutputSystem::insert` and `capture_selection` block on the worker, which
   round-trips keystrokes through the **main** thread. Calling them on the
   main thread deadlocks. IPC commands run on the main thread — any new
   command that touches selection/paste must offload via `spawn_blocking`
   exactly like `start_refine_selection` does.
3. **HUD window is never hidden or shown (`hud.rs`).** It is created once —
   transparent, click-through, all workspaces — and the webview content fades
   instead. Tauri bug #14102: even `focusable: false` windows steal focus on
   show. The changes overlay (`changes.rs`) follows the same pattern with
   `set_ignore_cursor_events` flips.
4. **Whisper context must be freed on exit (`main.rs`).** macOS quit paths
   end in `exit()` without running drops; ggml's Metal teardown then aborts
   on resident buffers. `RunEvent::Exit` → `stt.unload()` handles it. Quit
   must go through `app.exit(0)` (as `tray.rs` does), never
   `std::process::exit`.
5. **Settings writes fan out.** Every backend-initiated settings change emits
   `SETTINGS_CHANGED_EVENT` so open webviews stay in sync. If you mutate
   settings in Rust and skip the emit, the UI silently shows stale state.
6. **Profile ids are filename stems.** `safe_id` in `profiles.rs` and
   `stt_profiles.rs` (kept in sync, both tested) is the only thing standing
   between a profile id and a path traversal. Any new file-backed store
   copies that pattern: id validation, 0600 on create, atomic tmp+rename
   write, corrupt files skipped never deleted.
7. **One LLM client.** Every provider goes through the OpenAI-compatible
   `/v1/chat/completions` client in `llm.rs` configured by the active
   profile; the Ollama-native API is used for model listing only. New
   providers are presets/profiles, not new code paths. Prompts wrap
   transcripts as data — keep them injection-resistant.

## macOS-only assumptions

The app is macOS-only by design (menu bar, Carbon hotkeys, CGEvent paste,
TCC permissions, Metal whisper). Non-macOS `cfg` branches exist only to keep
check builds compiling, not to work:

- `permissions.rs` — TCC microphone/Accessibility, `x-apple.systempreferences:`
  deep links.
- `output.rs` — CGEvent keystroke synthesis, TIS keyboard-layout handling.
- `shortcuts.rs` — Carbon `RegisterEventHotKey` via the global-shortcut
  plugin (chosen because it needs no Accessibility permission).
- `hud.rs` / `changes.rs` — `NSPanel` reclassing, collection behaviors,
  `unsafe` blocks with documented invariants.
- `apps.rs` — `NSWorkspace.frontmostApplication`.
- `commands.rs` — Dock policy (`ActivationPolicy`).

New platform-touching code goes behind `#[cfg(target_os = "macos")]` with a
compiling no-op fallback, and the macOS quirk it works around gets a comment
naming the constraint (and the Tauri/macOS issue number when one exists).

## Privacy boundaries (the product invariant)

- Audio exists only in memory, only during a recording. It is never written
  to disk, never put in an event payload, and leaves the process only through
  `cloud_stt.rs` after the per-profile consent gate.
- Network code lives in exactly three modules — `llm.rs`, `models.rs`,
  `cloud_stt.rs` — enforced by `check-privacy.mjs`. A fourth network module
  is a privacy-policy change: update PRIVACY.md and the script's allowlist in
  the same PR, and expect the review to focus on it.
- Transcripts persist only behind `historyEnabled` (default off). Counters
  and aggregates (`stats.rs`, `suggestions.rs`, tip counters) are in-memory
  and reset on quit — keep them counts, never logs.
- No telemetry. Not "anonymized telemetry", not "crash reporting we'll remove
  later". The manifests are scanned for known SDK names.
