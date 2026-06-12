# Velata Roadmap

The MVP ships hold-to-talk local dictation, modes, a personal dictionary, selected-text
polish, and optional AI cleanup. Everything below is ordered by expected impact; nothing is
scheduled until someone owns it.

## Near term (polish the core)

- **Signed & notarized releases** — Apple Developer ID signing + notarization in
  `release.yml`, plus a Homebrew cask. Today users build from source or right-click → Open.
- **API keys in the macOS Keychain** — replace plain-JSON storage (documented tradeoff in
  ARCHITECTURE.md) with `security`-framework storage.
- **Streaming-feel latency** — start whisper on the buffered audio as soon as the key is
  released today; next: chunked inference while still recording so long dictations finish
  near-instantly on release.
- **Audio cues** — optional start/stop sounds (HUD-only feedback today).
- **Better silence/VAD gating** — energy hysteresis instead of a fixed RMS threshold; skip
  transcription earlier and more reliably.
- **Menu-bar recording indicator** — swap the template icon while recording (known
  `set_icon_as_template` reset bug needs the documented workaround).

## Medium term (feature depth)

- **Per-app modes** — auto-switch mode by frontmost app bundle id (`{bundleId → modeId}`);
  the single most-cited gap between open-source tools and Superwhisper.
- **Opt-in history** — local, searchable, raw-vs-final diff view, one-click re-copy; default
  off to preserve the "nothing persisted" story.
- **Command mode** — "select the last sentence", "press enter": a constrained voice-command
  grammar executed via synthetic keystrokes.
- **Spoken punctuation & casing commands** — "comma", "all caps on/off", configurable per mode.
- **Dictionary import/export** — CSV round-trip (Superwhisper users ask for this constantly).
- **`Fn`-key push-to-talk (opt-in)** — CGEventTap + Input Monitoring permission, clearly
  explained; default stays permission-free.
- **Typed-text insert option** — `enigo.text()` per-character mode for apps that block paste.

## Longer term (platform & engine)

- **Windows, then Linux** — the Rust core is portable by construction (cpal/enigo/arboard are
  cross-platform; `permissions.rs` and paste keystrokes are the macOS-specific seams).
- **Alternative STT engines** — NVIDIA Parakeet (ONNX) for sub-second latency;
  Apple SpeechAnalyzer via a Swift plugin on macOS 26+; engine choice behind the existing
  `stt.rs` interface.
- **CoreML encoder** — whisper.cpp's CoreML path (~3× encoder speedup on ANE) once the
  model-conversion step can be automated for users.
- **Whisper-server sidecar option** — share one inference server across apps via the
  OpenAI-compatible audio endpoint.
- **Translation mode** — dictate in one language, insert in another (whisper translate +
  LLM pass).

## Explicit non-goals

Team accounts, cloud sync, telemetry of any kind, a hosted backend, mobile apps, marketplace.
If a feature requires Velata to run servers, it does not belong in this repository.
