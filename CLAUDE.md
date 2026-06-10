# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

OpenFlow is a local-first macOS dictation app (Tauri 2 + Rust + whisper.cpp + React). Hold `⌥Space` → record → on-device whisper STT → rules/LLM cleanup → paste into the active app. A second hotkey (`⌥⇧Space`) rewrites selected text from a spoken instruction. Privacy is the product: audio never leaves the machine, there is no telemetry code, and PRs that add telemetry, default-on network calls, or audio persistence will not be merged. Cloud anything must be opt-in and BYO-key.

## Commands

```sh
pnpm install            # once; first Rust build also compiles whisper.cpp (~2–4 min, needs CMake)
pnpm dev                # tauri dev: Vite + Rust, hot reload
pnpm tauri build        # release DMG → apps/desktop/src-tauri/target/release/bundle/dmg/
```

Checks — CI (`.github/workflows/ci.yml`) runs exactly these; all must pass before a PR:

```sh
pnpm lint && pnpm format:check && pnpm typecheck && pnpm -r test     # TS (repo root)
cd apps/desktop/src-tauri
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Single tests:

- TS: `pnpm --filter @openflow/core test hotkey` (file filter) or append `-t 'name'`; same with `--filter @openflow/desktop`.
- Rust: `cargo test resample::` (module) or `cargo test <substring>` from `apps/desktop/src-tauri/`.
- Real-model STT integration test (ignored by default): `OPENFLOW_TEST_MODEL=/path/to/ggml-tiny.en.bin cargo test -- --ignored`.

If `cargo` is missing in non-interactive shells, use `$HOME/.cargo/bin/cargo`.

The remote is `github.com/KiwiGaze/openflow`; the `github.com/openflow-app/openflow` URLs in docs and Cargo.toml are stale placeholders.

## Architecture

Full details in `docs/ARCHITECTURE.md` (tradeoffs + module map) and `docs/PRD.md`. The short version:

- **Process model.** One Tauri 2 process. The Rust core (`apps/desktop/src-tauri/src/`) owns everything that touches the OS or heavy compute — audio, whisper (linked in-process via whisper-rs, Metal), LLM HTTP, clipboard/paste, hotkeys, tray, settings. Two small React webviews are pure UI over Tauri IPC: the settings app (`index.html` → `src/app/App.tsx`) and the HUD overlay (`hud.html` → `src/app/Hud.tsx`).
- **The IPC contract is hand-mirrored.** Serde structs in Rust (`settings.rs`, `pipeline.rs`, `models.rs`, …) ↔ `packages/core/src/types.ts`, camelCase on the wire. Any change to a struct that crosses IPC must update both sides in the same PR. There is no codegen — keep the mirror exact.
- **Pipeline.** `pipeline.rs` is the state machine: `idle → recording → transcribing → (refining) → inserting → idle`, with `notice`/`error` auto-clearing after 4 s. Cancellation is race-free via a monotonically increasing generation counter: every async stage re-checks the generation before publishing; `cancel()` just bumps it. Preserve this pattern when touching pipeline stages.
- **Stage modules.** `shortcuts.rs` (Carbon hotkeys, no permissions needed) → `audio.rs` (cpal capture, mono downmix, RMS level atomic) → `resample.rs` (hand-rolled windowed-sinc → 16 kHz) → `stt.rs` (whisper in `spawn_blocking`; dictionary passed as `initial_prompt`) → `text.rs` (artifact stripping, rules cleanup or LLM handoff, dictionary replacements) → `output.rs` (clipboard write + synthetic ⌘V + clipboard restore; degrades to clipboard-only without Accessibility).
- **Threading is deliberate.** `cpal::Stream` is `!Send` → it lives on a dedicated `openflow-audio` thread. `Enigo` + `arboard` live on a dedicated `openflow-output` thread (CGEvent thread-affinity; also serializes paste/copy sequences). Whisper runs in `spawn_blocking` behind a `Mutex` (context reused — loading maps hundreds of MB). Cross-thread communication is `std::sync::mpsc` with reply channels; shared state is only the settings `RwLock`, pipeline `Mutex`, and the audio-level atomic. Don't move these onto async executors.
- **HUD invariant.** The HUD window is created once at startup — transparent, click-through, all workspaces — and is **never hidden or shown again** (Tauri bug #14102: even `focusable:false` windows steal focus when shown). The webview content fades in/out on pipeline events instead. Don't "fix" this by hiding/showing the window.
- **Persistence.** One JSON file (`~/Library/Application Support/app.openflow.desktop/settings.json`), camelCase, schema-versioned, atomically written. Models are ggml files under `<app-data>/models/`. Nothing else is persisted — no audio, no transcripts, no history; that is a privacy feature, not an omission.
- **One LLM client for all providers.** Everything (Ollama, OpenAI, Groq, OpenRouter, LM Studio, llama.cpp) goes through one OpenAI-compatible `/v1/chat/completions` client in `llm.rs`; Ollama-native API is used only for model listing. Transcripts are wrapped in prompts that treat content as data — keep prompts injection-resistant.

## Conventions

- **Rust:** clippy clean with `-D warnings`; no `unwrap()` outside tests (poisoned-lock `expect()` is the accepted exception); failures become user-readable `AppError`s that surface as HUD notices — never silently drop dictation output (worst case it lands on the clipboard).
- **TypeScript:** strict, no `any`, explicit return types on exported functions.
- **Comments** explain _why_ (invariants, OS quirks), not _what_.
- **Commits:** conventional-ish (`feat:`, `fix:`, `docs:`, `chore:`), scope when it helps (`feat(stt): …`).
- Keep the dictation critical path fast and dependency-light.
- Add tests for logic changes (text processing, settings, IPC shapes). GUI, TCC permission flows, and real paste behavior are manual-only — checklist in `docs/DEVELOPMENT.md`.

## macOS development notes

- Under `pnpm dev`, TCC permission grants (Microphone, Accessibility) attach to your **terminal**, not an app bundle; the bundled app asks for its own grants. Without Accessibility, paste degrades to clipboard-only.
- Logs: terminal + `~/Library/Logs/app.openflow.desktop/`.
