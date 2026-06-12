# OpenFlow Privacy Statement

OpenFlow is built so that privacy is a property of the architecture, not a promise in a policy.
This document states exactly what data exists, where it goes, and how to verify it.

## Your voice

- Audio is captured **only while the hotkey is held** (or hands-free mode is latched), kept in
  memory, and discarded immediately after transcription.
- Audio is **never written to disk** — there is no code path that does that.
- **By default** audio is **never transmitted**: transcription runs entirely on your Mac via
  whisper.cpp. The only way audio leaves the device is if you add a **cloud speech engine**
  yourself (Settings → Models, bring your own key). Turning one on uploads each recording to that
  provider for transcription, and OpenFlow shows a consent dialog naming what is sent before it
  ever does. The on-device engine stays the default; OpenFlow stores nothing.

## Your text

- Transcripts are processed in memory and inserted into the app you're using. **By default**
  OpenFlow keeps no history; only the most recent result is held in memory (for "Copy last
  dictation") until the app quits. You can opt into a local dictation **history** (General → Save
  history, default off) — text only, stored only on this Mac, capped and clearable anytime.
- If — and only if — you configure an AI provider, the **text** of a transcript (plus your mode
  prompt, dictionary words, and, for selection polish, the selected text) is sent to that provider:
  - **Ollama / llama.cpp / LM Studio:** stays on your machine (localhost) unless you point the
    base URL elsewhere.
  - **OpenAI-compatible cloud (your API key):** goes to the base URL you set, under that
    provider's terms. Audio is never sent — only text.
- With the provider set to **None** (the default), nothing is ever transmitted.

## Network connections — the complete list

1. `huggingface.co` — downloading the speech model you pick, when you pick it.
2. Your configured AI provider's base URL — only when a mode uses AI or you run a polish/test.
3. A cloud speech engine's base URL — only if you added one and selected it; this uploads your
   **audio** (the one exception to "audio never leaves"), after the consent dialog.

There is no telemetry, no crash reporting, no update pinging, no analytics SDK. You can verify
with Little Snitch/LuLu, or block the app's network entirely after the model download —
dictation keeps working.

## What is stored on disk

| Data                                                   | Location                                                           | Notes                                                                                                                   |
| ------------------------------------------------------ | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Settings (hotkeys, modes, dictionary, provider config) | `~/Library/Application Support/app.openflow.desktop/settings.json` | Plain JSON you can read and delete                                                                                      |
| API key (if you set one)                               | same file                                                          | **Stored in plain text** in the MVP — Keychain storage is on the roadmap. Prefer local providers if this matters to you |
| Speech models                                          | `~/Library/Application Support/app.openflow.desktop/models/`       | Public model files, deletable in Settings                                                                               |
| Logs                                                   | `~/Library/Logs/app.openflow.desktop/`                             | Operational messages; never transcript contents or audio                                                                |

## Permissions OpenFlow asks for

- **Microphone** — to hear you while the hotkey is held.
- **Accessibility** — to simulate ⌘V (paste) and ⌘C (capture a selection for rewriting).
  Optional: without it, results are copied to your clipboard instead.

It does not request Input Monitoring, Screen Recording, Full Disk Access, or Contacts/Calendar
anything.

## Uninstalling

Delete the app, then remove the two folders listed above. That is everything OpenFlow ever
created.
