<p align="center">
  <img src="apps/desktop/app-icon.png" width="96" alt="OpenFlow icon" />
</p>

<h1 align="center">OpenFlow</h1>

<p align="center">
  <strong>Local-first AI voice input for macOS.</strong><br/>
  Hold a key, speak naturally, release — clean text appears in whatever app you're using.
</p>

<p align="center">
  <a href="LICENSE">MIT</a> ·
  macOS 13+ (Apple Silicon recommended) ·
  Tauri + Rust + whisper.cpp ·
  <a href="docs/PRD.md">PRD</a> ·
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="ROADMAP.md">Roadmap</a>
</p>

---

## What it does

- **Dictate anywhere.** Hold `⌥ Space`, talk, release. OpenFlow transcribes your speech
  _on-device_ with whisper.cpp (Metal-accelerated), removes the "um"s, fixes punctuation, and
  pastes the result at your cursor. A quick tap latches hands-free mode.
- **Polish selected text with one keystroke.** Select text in any app, tap `⌥⇧ P`, and the
  selection is replaced with a grammar-, spelling-, and clarity-fixed version. No voice needed.
- **Rewrite selected text by voice.** Select text in any app, hold `⌥⇧ Space`, and say what you
  want: _"make this more polite"_, _"fix the grammar"_, _"turn this into bullet points"_.
- **Modes** shape the output — Standard, Email, Notes, Literal, or your own custom prompts.
- **Personal dictionary** teaches it your names, products, and jargon ("open flow" → "OpenFlow").
- **Optional AI polish.** Plug in [Ollama](https://ollama.com) for fully-local LLM cleanup, or
  bring your own key for any OpenAI-compatible API (OpenAI, Groq, OpenRouter, LM Studio,
  llama.cpp). Or use neither — rules-based cleanup works offline with zero setup.

## Why OpenFlow

|                  | Cloud dictation apps | OpenFlow                                         |
| ---------------- | -------------------- | ------------------------------------------------ |
| Where audio goes | Their servers        | **On your Mac by default** (cloud STT is opt-in) |
| Price            | $8–15/month          | **Free, MIT-licensed**                           |
| Word limits      | Capped free tiers    | **Unlimited**                                    |
| Auditability     | Trust the policy     | **Read the source, firewall the app**            |

Privacy here is architectural, not contractual: by default transcription is in-process
whisper.cpp, audio lives only in memory, and there is no telemetry code at all. After downloading
a model you can block the app's network access entirely — dictation keeps working. Cloud speech
engines are off unless you add one yourself (bring your own key); turning one on uploads your
audio to that provider, and OpenFlow shows a consent dialog before it ever does.

## Install

**Build from source** (no signed releases yet — see the [roadmap](ROADMAP.md)):

```sh
git clone https://github.com/KiwiGaze/openflow.git
cd openflow
pnpm install
pnpm tauri build        # produces apps/desktop/src-tauri/target/release/bundle/dmg/
```

Prerequisites: Xcode Command Line Tools, Rust (stable), Node 22+, pnpm 10+, CMake
(`brew install cmake`). Details in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## First run

1. Launch OpenFlow — it lives in the **menu bar** (no Dock icon).
2. The onboarding asks for **Microphone** access (to hear you) and **Accessibility** access
   (to paste with ⌘V — skippable; results go to the clipboard instead).
3. Pick a speech model. They download once from Hugging Face, then run offline:

   | Model                      | Size   | Good for                             |
   | -------------------------- | ------ | ------------------------------------ |
   | Base (English)             | 148 MB | Fast, decent accuracy — a good start |
   | Small (English)            | 488 MB | Balanced                             |
   | Large v3 Turbo (quantized) | 574 MB | Best quality on Apple Silicon        |

4. Click into any text field, hold `⌥ Space`, say something, release.

## Usage notes

- **Switch modes** from the menu-bar icon (Standard / Email / Notes / Literal / custom).
- **Hands-free:** tap the hotkey instead of holding; tap again to stop.
- **Recover a result:** menu bar → _Copy Last Result_.
- **AI profiles:** Settings → Models. Each profile is a connection (provider, endpoint, model)
  stored as a file under the app's `profiles/` folder; exactly one is active for refinement.
  With Ollama, `ollama pull qwen2.5:3b` is a fast, high-quality default for cleanup.
- Dictation works without any AI profile; _polish_ and _rewrite selection_ require one.

## Development

```sh
pnpm install
pnpm dev            # tauri dev: Vite + Rust with hot reload
pnpm check:all      # every check CI runs: TS + Rust + IPC-contract + privacy gates
```

The monorepo: `packages/core` (shared TS contract + utils), `apps/desktop` (React UI +
`src-tauri` Rust core). Start with [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), then
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). Contributions welcome — see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Privacy

See [PRIVACY.md](PRIVACY.md) for the complete data-flow statement, including exactly which
network destinations exist (model downloads; your optional LLM endpoint; and, only if you add
one, an opt-in cloud speech engine) and what each receives.

## License

[MIT](LICENSE) © OpenFlow contributors. whisper.cpp is MIT-licensed by Georgi Gerganov and
contributors; Whisper models are released by OpenAI under MIT.
