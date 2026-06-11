# 00 — Current state: product inventory and design baseline

Status: reference document. Written 2026-06-11 against `main` (cd63494) plus the in-flight
Refine work (`docs/REFINE.md`, partially implemented). Every other document in `docs/design/`
treats this file as the factual baseline. If this file and the code disagree, the code wins.

## 1. What OpenFlow is

A local-first macOS menu-bar dictation app (Tauri 2 + Rust + whisper.cpp + React).

- **Dictate:** hold `⌥Space`, speak, release → on-device transcription → cleanup (rules or
  LLM) → text pasted into the active app. A tap < 350 ms latches hands-free mode.
- **Rewrite selection:** select text anywhere, hold `⌥⇧Space`, speak an instruction
  ("make this polite") → LLM rewrites the selection in place.
- **Polish selection** (in flight): tap `⌥⇧P` → fixes grammar/clarity of the selection with a
  built-in instruction, no voice involved.

Privacy is the product: audio never leaves the machine, STT is in-process whisper.cpp, no
telemetry exists, cloud LLMs are opt-in BYO-key and receive text only. The app works fully
offline after a model download.

## 2. Surfaces

| Surface         | File(s)                                | What it shows                                                                                               |
| --------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Menu-bar tray   | `tray.rs`                              | Mode list (radio), Copy Last Result, Settings…, Quit                                                        |
| HUD pill        | `Hud.tsx`, `hudState.ts`, `hud.rs`     | Level bars + label while recording/transcribing/refining/inserting; notices and errors auto-clear after 4 s |
| Settings window | `App.tsx` + `tabs/*.tsx`               | Sidebar tabs: General, Modes, Dictionary, AI Provider (→ Refine, in flight), About                          |
| Onboarding      | `Onboarding.tsx`                       | 5 steps: Welcome → Microphone → Accessibility → Speech model → Try it                                       |
| Global hotkeys  | `shortcuts.rs` (Carbon, no permission) | `⌥Space` dictation, `⌥⇧Space` rewrite, `⌥⇧P` polish (in flight)                                             |

There is no Dock icon (Accessory activation policy); the settings window temporarily switches
the app to Regular so it can take focus.

## 3. Settings window inventory (committed baseline)

### General tab (`GeneralTab.tsx`)

| Card               | Rows                                                                                                                                                                                                 |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hotkeys            | Dictation (recorder), Dictation style (hold / toggle select), Rewrite selection (recorder)                                                                                                           |
| Speech recognition | Model list — radio selects active (disabled until installed), download/cancel/delete, size + description, multilingual badge; Spoken language select (14 options, "English-only models ignore this") |
| Output             | Insert method (paste / clipboard select), Restore clipboard (toggle), Launch at login (toggle)                                                                                                       |
| Last result        | Appears only after a dictation: final text, raw transcript (when different), Copy button                                                                                                             |

### Modes tab (`ModesTab.tsx`)

- List of modes; radio = active mode, click row = select for editing (two different gestures
  on one row). Badges: `AI` when `usesLlm`, `built-in`.
- Editor card: Name (custom only), Uses AI (read-only text for built-ins, toggle for custom),
  Prompt textarea (read-only for built-ins), Duplicate, Delete (custom only).
- "New mode" creates "New mode" with a generic prompt; built-ins are read-only but duplicable.

### Dictionary tab (`DictionaryTab.tsx`)

- One card: explanation line, `from → to` add form, list with Remove buttons.
- Empty state is a single line: "No entries yet."

### AI Provider tab (`ProviderTab.tsx`, committed — being replaced by Refine tab)

- Provider select: None / Ollama (local) / OpenAI-compatible API (your key).
- Conditional fields: Base URL, API key (openaiCompatible only), Model (+ "List installed
  models" chips for Ollama), Timeout, Test connection button with inline result.
- Cloud privacy note shown for openaiCompatible.

### About tab

- Version, data directory, config path, links.

## 4. Onboarding inventory (committed)

Five fixed steps, dot indicators, Back/Continue, "Skip setup" always visible.

1. **Welcome** — value line + 3-bullet privacy list. Shows the current dictation hotkey.
2. **Microphone** — status badge, Allow button (or "Open System Settings" if denied), a hint
   about `tauri dev` terminal permissions.
3. **Accessibility** — explanation that paste = synthetic ⌘V, Grant + Open System Settings,
   "skip → clipboard instead".
4. **Speech model** — 3 starter models (base.en 148 MB / small.en 488 MB / large-v3-turbo-q5_0
   574 MB), radio + download per row; Continue gated on the selected model being installed.
5. **Try it** — instructions to click into any text field and dictate; shows live pipeline
   status badge and the last result text.

Notable: onboarding never mentions the rewrite hotkey, modes, the dictionary, or AI
refinement. Skip is silent (no summary of what was skipped). There is no way to re-run it.

## 5. Pipeline and failure policy

```
idle → recording → transcribing → (refining) → inserting → idle
                                  notice / error (auto-clear 4 s)
```

- Jobs: `dictation`, `refineSelection`, `polishSelection` (in flight; skips recording and
  transcribing entirely).
- Cancellation: generation counter; `Esc` is not bound — cancel happens via tray quit or a new
  job only. (`cancel_dictation` exists as a command.)
- Dictation failure policy: LLM error → rules-cleaned transcript + notice (never lose text);
  paste failure → text stays on clipboard + notice.
- Selection jobs: no fallback on LLM error — selection untouched, error shown.
- Recording cap: 5 minutes. Silence/no-speech → notice "Didn't catch anything".
- HUD labels: Listening… / Listening for instruction… / Transcribing… / Polishing… /
  Inserting… (job-aware labels for refining are part of the in-flight work).

## 6. Data model

### Committed `Settings` (schema v1)

`version, dictationHotkey, dictationHotkeyBehavior (hold|toggle), refineHotkey, activeModeId,
modes[], dictionary[], sttModelId, language, llm{provider: none|ollama|openaiCompatible,
baseUrl, apiKey, model, timeoutSecs}, insertMethod (paste|clipboard), restoreClipboard,
launchAtLogin, onboardingCompleted`

### In-flight v2 (REFINE.md, largely implemented in Rust)

- `llm` block removed → **AI profiles**: one JSON file per profile in `<app-data>/profiles/`,
  `activeLlmProfileId` pointer in settings (`""` = No AI). Migration is automatic and
  self-erasing.
- New: `polishHotkey` (default `Alt+Shift+P`), `refineAfterDictation` (default `true`) — a
  master kill switch ANDed with per-mode `usesLlm`.
- `LlmProfile`: `{version, id (= filename stem), name, provider: ollama|openaiCompatible,
baseUrl, apiKey, model, timeoutSecs}` — 0600 perms, atomic writes, corrupt files skipped
  never deleted, hand-dropped files appear on next scan ("Show in Finder" is the
  import/export story).
- Persistence beyond settings/models/profiles: **none** — no audio, no transcripts, no
  history (privacy feature, not an omission).

### Modes (committed)

`{id, name, builtIn, usesLlm, prompt}`. Built-ins: Standard (light cleanup), Email, Notes
(bullets), Literal (`usesLlm: false`, raw + dictionary only). Modes that use AI degrade to
rules-based cleanup when no provider is configured. Active mode switched in tray or Modes tab.

### STT models (committed)

Static registry in `models.rs` (ggml whisper files, Hugging Face URLs); downloaded to
`<app-data>/models/`; radio-select active; `language` (`auto` or ISO 639-1) passed to whisper.
There is **one** STT engine (whisper.cpp in-process) — no engine abstraction in the UI.

## 7. Canonical vocabulary (do not drift)

| Term                  | Means                                                                                             | Never call it                         |
| --------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------- |
| **Mode**              | A dictation output style (Standard, Email, Notes, Literal, custom)                                | profile, preset, prompt               |
| **AI profile**        | A named LLM connection (provider + URL + key + model + timeout), file-backed                      | account, provider (alone), mode       |
| **Polish selection**  | Tap `⌥⇧P`: fix the selection with the built-in instruction, no voice                              | a mode; "polish mode"                 |
| **Rewrite selection** | Hold `⌥⇧Space`: speak an instruction, selection is rewritten                                      | a mode; "refine selection" in UI copy |
| **Refine**            | Umbrella for everything that sends text through the LLM (dictation cleanup pass, Polish, Rewrite) | —                                     |
| **Dictionary**        | `from → to` replacement pairs + STT vocabulary hints                                              | vocabulary, glossary (in UI)          |
| **Insert**            | The output step (paste or clipboard)                                                              | type, send                            |

REFINE.md explicitly flags the naming hazard: "mode" must keep meaning dictation style only.

## 8. Hard constraints any design must respect

1. **Privacy invariants** — no telemetry, no default-on network, no audio persistence, cloud
   is opt-in BYO-key, text-only to LLM endpoints. PRs violating this are rejected outright.
2. **HUD window is never hidden/shown** after creation (Tauri #14102 focus-steal bug);
   visibility is faked by fading webview content. Designs may restyle the HUD content freely.
3. **Hand-mirrored IPC contract** — Rust serde structs ↔ `packages/core/src/types.ts`,
   camelCase; both sides change in the same PR; no codegen.
4. **Threading model is fixed** — audio on its own thread, output (Enigo/arboard) on its own
   thread, whisper in `spawn_blocking`. UI designs must not assume new long-lived async work
   on the UI thread.
5. **Critical path stays fast and dependency-light** — no heavyweight UI frameworks, no
   blocking network on the dictation path.
6. **Failure policy** — dictation output is never silently dropped; selection jobs never
   replace the selection on error.
7. **Settings are one JSON file** (plus profile files); schema-versioned with migrations;
   anything new must fit this persistence story or argue hard for its own file.
8. **No accounts, no cloud sync, no hosted backend, no marketplace** (ROADMAP non-goals).

## 9. Existing roadmap themes (ROADMAP.md, abridged)

Near: signed releases, Keychain for API keys, streaming-feel latency, audio cues, better VAD,
menu-bar recording indicator. Medium: per-app modes, opt-in history, command mode, spoken
punctuation, dictionary import/export, Fn push-to-talk, typed-text insert. Long: Windows/Linux,
alternative STT engines (Parakeet, Apple SpeechAnalyzer), CoreML encoder, whisper-server
sidecar, translation mode.

## 10. Design principles already established (keep them)

- "Every feature is one flat row — a name that states what it does, a control, at most one
  short hint line" (REFINE.md settings principle).
- Radio-selects-active + click-row-to-edit list idiom (models, modes, profiles).
- The tray is the quick-switch surface; the settings window is the configuration surface; the
  HUD is feedback-only.
- Local/cloud distinction is derived from facts (localhost base URL ⇒ `local` badge), not from
  provider labels — the privacy story must be visible at a glance.
- Defaults must make the product fully usable with zero configuration and zero network.
