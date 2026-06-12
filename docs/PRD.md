# Velata — Product Requirements

**Status:** MVP shipped · **Platform:** macOS 13+ (Apple Silicon first) · **License:** MIT

## 1. Problem

Speaking is 3–4× faster than typing, but raw speech-to-text output is unusable as written text:
it is full of filler words, false starts, and missing punctuation. Commercial tools that fix this
(Wispr Flow, Superwhisper) are closed-source, subscription-priced, and — in Wispr Flow's case —
send every utterance to the cloud. Privacy-conscious users, developers, and non-native speakers
have no trustworthy, free, local option with the same "hold a key, get clean text" experience.

## 2. Product

Velata is a local-first macOS menu-bar app:

1. **Dictate anywhere.** Hold a global hotkey (default `⌥ Space`), speak, release. The speech is
   transcribed on-device (whisper.cpp + Metal), cleaned up, and pasted into the active app.
2. **Polish anything.** Select text in any app and press a polish shortcut (default `⌥⇧ P`) —
   the built-in fix-grammar prompt or one of your own — and the selection is replaced with the
   polished text.

### Positioning

|              | Wispr Flow                      | Superwhisper             | **Velata**                                   |
| ------------ | ------------------------------- | ------------------------ | -------------------------------------------- |
| STT location | Cloud only                      | Local or cloud           | **Local only**                               |
| AI cleanup   | Cloud, always on                | Cloud, BYO key           | **Optional: local Ollama, BYO key, or none** |
| Price        | $15/mo, capped free tier        | $8.49/mo / $249 lifetime | **Free, MIT**                                |
| Source       | Closed                          | Closed                   | **Open**                                     |
| Runtime      | Electron (~800 MB RAM reported) | Native Swift             | **Tauri/Rust (small footprint)**             |

The wedge: _verifiable_ privacy (audio cannot leave the machine — by architecture, not policy),
zero cost for unlimited local dictation, and a hackable codebase.

## 3. Users

- **The writer-by-voice** — drafts emails/docs/messages by speaking; wants clean prose, not
  transcripts.
- **The privacy-required professional** — legal/health/security contexts where audio must not
  leave the device; needs to be able to verify that claim (firewall the app, read the source).
- **The non-native English speaker** — speaks comfortably but wants grammar and phrasing polished
  on the way out; the polish hotkey doubles as an inline editor for typed text.
- **The developer/tinkerer** — wants a dictation engine they can extend (modes are prompts;
  providers are any OpenAI-compatible endpoint).

## 4. Core UX flows

### 4.1 Dictation (critical path)

```
hold ⌥Space ─► HUD shows "Listening…" + level meter
speak        ─► audio buffered in memory only
release      ─► HUD: Transcribing… → (Polishing…) → text pasted at cursor
```

- Hold-to-talk by default. A **tap shorter than 350 ms latches hands-free mode** (press again to
  stop) — the convention users know from Superwhisper/Wispr Flow.
- A `toggle` behavior setting replaces hold semantics entirely for users who prefer it.
- Recording is capped at 5 minutes.
- If anything fails, the dictation is **never lost silently**: on LLM failure the rules-cleaned
  transcript is inserted; on paste failure the text is left on the clipboard and the HUD says so.
- "Copy Last Result" lives in the tray menu as a recovery affordance.

### 4.2 Selected-text polish

```
select text ─► tap ⌥⇧P (or a transform's own hotkey)
            ─► selection captured via ⌘C round-trip
            ─► LLM applies the prompt ─► result pasted over the selection
```

- Requires an AI provider; without one the HUD explains how to set it up.
- The built-in Polish prompt is: _fix grammar, spelling, clarity; keep meaning, tone, and
  language_. Custom prompts (transforms) carry their own instruction and hotkey.
- No fallback insert on failure — replacing a user's selection with something wrong is worse than
  doing nothing.

### 4.3 Modes

A **mode** decides how the transcript becomes written text. Built-ins:

| Mode                 | Uses AI  | Behavior                                              |
| -------------------- | -------- | ----------------------------------------------------- |
| Standard _(default)_ | optional | Filler removal, self-correction handling, punctuation |
| Email                | optional | Polite, well-structured prose                         |
| Notes                | optional | Concise `- ` bullet points                            |
| Literal              | never    | Raw transcript + dictionary only (for code, commands) |

- "Uses AI" modes degrade gracefully to deterministic rules-based cleanup when no provider is
  configured — the core product works with zero setup and zero network.
- Users can create custom modes (name + prompt); built-ins are read-only but duplicable.
- Active mode is switched from the tray menu.

### 4.4 Personal dictionary

`from → to` pairs ("open flow" → "Velata"). Applied as whole-word, case-insensitive
replacements after transcription, fed to whisper as a vocabulary-biasing initial prompt, and
listed in LLM prompts so polish preserves the exact spellings. This is table-stakes in every
competing product and the first thing users notice missing.

### 4.5 Onboarding

Five screens: Welcome (privacy promise) → Microphone permission → Accessibility permission
(skippable; falls back to clipboard-only) → model pick & download (Base English 148 MB /
Small English 488 MB / Large-v3-turbo quantized 574 MB) → live try-it. Skippable at any point.

## 5. Privacy requirements (non-negotiable)

- Audio is held in memory only, never written to disk, never transmitted.
- STT runs on-device. The app is fully functional with the network blocked (after model download).
- No telemetry, no accounts, no auto-update phone-home.
- Cloud LLMs are opt-in, BYO-key, and receive **text only** — never audio. The UI says so at the
  point of configuration.
- History is not persisted in the MVP; only the last result is kept in memory.

## 6. Scope

### In (MVP)

Menu-bar app, global hotkeys (hold/toggle/tap-latch), local whisper.cpp transcription with model
manager, rules-based cleanup, optional LLM polish (Ollama / OpenAI-compatible), modes,
personal dictionary, selected-text polish, paste-with-clipboard-restore, settings UI,
onboarding, launch-at-login, single-instance.

### Out (deliberately)

Real-time streaming transcription, Windows/Linux (architected for, not shipped), mobile, team
accounts, cloud sync, persistent history, app-specific auto mode switching, marketplace,
custom STT fine-tuning. See `ROADMAP.md`.

## 7. Success criteria

- Cold dictation (10 s utterance, base.en, M-series) inserts text in **< 2 s** after release.
- Zero network connections during dictation with provider = none (verifiable with Little Snitch).
- A first-time user reaches a successful dictation in **< 3 minutes** including model download
  on a fast connection.
- `pnpm lint && pnpm typecheck && pnpm -r test && cargo test && cargo clippy` all pass in CI.

## 8. Key product decisions and why

| Decision                                                  | Rationale                                                                                                                                                                            |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `⌥ Space` default, not `Fn`                               | `Fn` requires a CGEventTap + Input Monitoring permission and breaks on external keyboards (top Wispr Flow complaint). `⌥ Space` is the Superwhisper convention and works everywhere. |
| Paste via ⌘V simulation, clipboard restored               | What both market leaders converged on; typing simulation is slower for long text. Clipboard-only fallback when Accessibility is missing.                                             |
| Default mode does _light_ cleanup, Literal one click away | Wispr's always-on rewriting is its most-loved and most-complained-about feature. Cleanup default + literal escape hatch covers both camps.                                           |
| LLM optional, never required                              | The "works with the network unplugged" story is the differentiation; an LLM dependency would kill it.                                                                                |
| Free unlimited local                                      | Local STT has zero marginal cost; a word cap (Wispr free tier) is the single most-mocked aspect of the incumbent.                                                                    |
