# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Velata is a local-first macOS dictation app (Tauri 2 + Rust + whisper.cpp + React). Hold the push-to-talk key (default `fn`; falls back to `⌥Space` without Input Monitoring) → record → on-device whisper STT → artifact strip + dictionary replacements → paste into the active app; an optional post-dictation **Prompt** (set via the HUD circle) can shape the transcript, but there is no always-on cleanup. A second hotkey (`⌥⇧P`, tap) polishes selected text with the built-in **Polish** prompt — no recording; custom prompts carry their own shortcuts. Privacy is the product: **by default** Velata transcribes entirely on your Mac and your audio never leaves the device. There is no telemetry code, and PRs that add telemetry, default-on network calls, or audio persistence will not be merged. Cloud anything is opt-in and BYO-key — cloud speech engines (off unless you add one) upload your audio to that provider, and Velata tells you clearly (a consent gate) before it ever does.

## Commands

```sh
pnpm install            # once; first Rust build also compiles whisper.cpp (~2–4 min, needs CMake)
pnpm dev                # tauri dev: Vite + Rust, hot reload
pnpm tauri build        # release DMG → apps/desktop/src-tauri/target/release/bundle/dmg/
```

Checks — `pnpm check:all` runs everything CI (`.github/workflows/ci.yml`) runs; all must pass before a PR:

```sh
pnpm check        # = pnpm -r build && check:ipc && check:privacy && lint && format:check && typecheck && pnpm -r test
pnpm check:rust   # = desktop vite build, then cargo fmt --check && clippy -D warnings && cargo test
pnpm check:all    # both, in CI order
```

`check:ipc` (scripts/check-ipc.mjs) fails on Rust↔TS IPC drift; `check:privacy` (scripts/check-privacy.mjs) fails on network code outside `llm.rs`/`models.rs`/`cloud_stt.rs`, webview network I/O, or telemetry deps. Build first matters: `@velata/desktop` imports `@velata/core` from its `dist/`.

Single tests:

- TS: `pnpm --filter @velata/core test hotkey` (file filter) or append `-t 'name'`; same with `--filter @velata/desktop`.
- Rust: `cargo test resample::` (module) or `cargo test <substring>` from `apps/desktop/src-tauri/`.
- Real-model STT integration test (ignored by default): `VELATA_TEST_MODEL=/path/to/ggml-tiny.en.bin cargo test -- --ignored`.

If `cargo` is missing in non-interactive shells, use `$HOME/.cargo/bin/cargo`.

The remote is `github.com/KiwiGaze/velata`.

## Architecture

Full details in `docs/ARCHITECTURE.md` (tradeoffs + module map) and `docs/PRD.md`. The short version:

- **Process model.** One Tauri 2 process. The Rust core (`apps/desktop/src-tauri/src/`) owns everything that touches the OS or heavy compute — audio, whisper (linked in-process via whisper-rs, Metal), LLM HTTP, clipboard/paste, hotkeys, tray, settings. The React webviews are pure UI over Tauri IPC, split into two decorated windows (both hide-on-close): the **App window** (label `main`, `index.html` → `src/app/App.tsx`; sidebar Home · Library · Transform · Scratchpad · ⚙ Settings) and a separate **Settings window** (label `settings`, its own `settings.html` MPA entry; sidebar Dictation · Speech · AI · General · About · ‹ Velata). Plus two non-decorated overlays: the always-present click-through HUD (`hud.html` → `src/app/Hud.tsx`) and the on-demand Scratchpad (`scratchpad.html`). The tray menu is **Open Velata · Copy last dictation · Quit** only.
- **The IPC contract is hand-mirrored.** Serde structs in Rust (`settings.rs`, `pipeline.rs`, `models.rs`, …) ↔ `packages/core/src/types.ts`, camelCase on the wire. Any change to a struct that crosses IPC must update both sides in the same PR. There is no codegen — keep the mirror exact; `pnpm check:ipc` catches name-level drift, and the full rules are in `docs/engineering/ipc-contract-conventions.md`.
- **Pipeline.** `pipeline.rs` is the state machine: `idle → recording → transcribing → (polishing) → inserting → idle`, with `notice`/`error` auto-clearing after 4 s. Cancellation is race-free via a monotonically increasing generation counter: every async stage re-checks the generation before publishing; `cancel()` just bumps it. Preserve this pattern when touching pipeline stages.
- **Stage modules.** `shortcuts.rs` (accelerator hotkeys via the global-shortcut plugin, no permissions needed) — the default `fn` push-to-talk gesture is instead observed by a listen-only **CGEventTap** in `fn_gesture.rs`, gated by Input Monitoring; without that grant it falls back to the `⌥Space` accelerator → `audio.rs` (cpal capture, mono downmix, RMS level atomic) → `resample.rs` (hand-rolled windowed-sinc → 16 kHz) → `stt.rs` (whisper in `spawn_blocking`; dictionary passed as `initial_prompt`) or `cloud_stt.rs` (opt-in, consent-gated upload to an STT profile from `stt_profiles.rs`) → `text.rs` (artifact stripping + dictionary/snippet replacements — the baseline; any further shaping is an optional Prompt) → `output.rs` (clipboard write + synthetic ⌘V + clipboard restore; degrades to clipboard-only without Accessibility). Around the pipeline: `commands.rs` (the whole IPC surface) + `state.rs` (shared `AppState`), `prompts.rs` (the one built-in Polish prompt + selection-rewrite prompt assembly), `apps.rs` (frontmost app name for the opt-in history row), `changes.rs` (diff overlay window), `stats.rs`/`suggestions.rs` (session-only counters), `history.rs` (opt-in history), `error.rs` (`AppError`).
- **Threading is deliberate.** `cpal::Stream` is `!Send` → it lives on a dedicated `velata-audio` thread. `Enigo` + `arboard` live on a dedicated `velata-output` thread (CGEvent thread-affinity; also serializes paste/copy sequences). Whisper runs in `spawn_blocking` behind a `Mutex` (context reused — loading maps hundreds of MB). Cross-thread communication is `std::sync::mpsc` with reply channels; shared state is only the settings `RwLock`, pipeline `Mutex`, and the audio-level atomic. Don't move these onto async executors.
- **HUD invariant.** The HUD window is created once at startup — transparent, click-through, all workspaces — and is **never hidden or shown again** (Tauri bug #14102: even `focusable:false` windows steal focus when shown). The webview content fades in/out on pipeline events instead. Don't "fix" this by hiding/showing the window.
- **Persistence.** One settings JSON file (`~/Library/Application Support/app.velata.desktop/settings.json`), camelCase, schema-versioned (currently v6), atomically written. Models are ggml files under `<app-data>/models/`; LLM profiles are one JSON file each under `<app-data>/profiles/` and cloud STT profiles under `<app-data>/stt-profiles/` (same pattern, both: filename stem = identity, 0600, corrupt files skipped never deleted; `activeLlmProfileId` in settings points at the active LLM profile, "" = no AI). Durable structured data lives in one SQLite file (`<app-data>/velata.db`, rusqlite bundled, WAL — `db.rs`): tables `history`, `notes`, `note_versions` (opt-in user content; soft-deleted / capped, never destroyed) and `insights_daily` (counts and dates only). Invariants: the app-data dir is 0700 and the db file 0600; schema migrates forward through `PRAGMA user_version`, each step inside one transaction; a corrupt/unopenable db is renamed aside to `velata.db.corrupt-<ts>` (never deleted) and a fresh one created. Audio is **never** persisted — that invariant is absolute. `insights_daily` is **always** written (counts/dates only — never words, never audio; no enable toggle, no reset), so privacy is unchanged: you cannot reconstruct a sentence from a tally. The user-content stores are off by default — history and transcripts behind `historyEnabled` (text only, capped, retention `historyRetentionDays`, clearable → `history.rs`), Scratchpad notes behind `scratchpadEnabled` (`notes.rs`). The `history` table keeps a vestigial `mode_id` column (written `""`, never read). Nothing else is written.
- **One LLM client for all providers.** Everything (Ollama, OpenAI, Groq, OpenRouter, LM Studio, llama.cpp) goes through one OpenAI-compatible `/v1/chat/completions` client in `llm.rs`, configured by the active profile; Ollama-native API is used only for model listing. Transcripts are wrapped in prompts that treat content as data — keep prompts injection-resistant.

## Conventions

- **Rust:** clippy clean with `-D warnings`; no `unwrap()` outside tests (poisoned-lock `expect()` is the accepted exception); failures become user-readable `AppError`s that surface as HUD notices — never silently drop dictation output (worst case it lands on the clipboard).
- **TypeScript:** strict, no `any`, explicit return types on exported functions.
- **Comments** explain _why_ (invariants, OS quirks), not _what_.
- **Commits:** conventional-ish (`feat:`, `fix:`, `docs:`, `chore:`), scope when it helps (`feat(stt): …`).
- Keep the dictation critical path fast and dependency-light.
- Add tests for logic changes (text processing, settings, IPC shapes). GUI, TCC permission flows, and real paste behavior are manual-only — checklist in `docs/DEVELOPMENT.md`.
- The full rulebook lives in `docs/engineering/` (layout/naming/comments → `monorepo-conventions.md`, ownership + break-prone constraints → `architecture-boundaries.md`, IPC → `ipc-contract-conventions.md`, what reviewers check → `review-checklist.md`). Task-specific procedures for agents: `.agents/skills/`.

## macOS development notes

- Under `pnpm dev`, TCC permission grants (Microphone, Accessibility) attach to your **terminal**, not an app bundle; the bundled app asks for its own grants. Without Accessibility, paste degrades to clipboard-only.
- Logs: terminal + `~/Library/Logs/app.velata.desktop/`.
