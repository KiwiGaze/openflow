# Developing OpenFlow

## Prerequisites

| Tool                     | Version | Install                                                             |
| ------------------------ | ------- | ------------------------------------------------------------------- |
| Xcode Command Line Tools | current | `xcode-select --install`                                            |
| Rust (stable)            | ≥ 1.82  | `curl https://sh.rustup.rs -sSf \| sh`                              |
| Node                     | ≥ 22    | `nvm install` (reads `.nvmrc`)                                      |
| pnpm                     | ≥ 10    | `corepack enable` or `npm i -g pnpm`                                |
| CMake                    | ≥ 3.20  | `brew install cmake` — **required**: whisper.cpp builds from source |

Apple Silicon strongly recommended; whisper runs with Metal acceleration automatically.

## Daily loop

```sh
pnpm install                 # once
pnpm dev                     # tauri dev: starts Vite, builds Rust, launches the app
```

- Frontend hot-reloads. Rust changes recompile on save and restart the app.
- The first Rust build compiles whisper.cpp (~2–4 min); afterwards it's incremental.
- Logs: terminal + `~/Library/Logs/app.openflow.desktop/`.

### Checks (run all before a PR)

```sh
pnpm check:all               # everything CI runs, in CI order — the one command you need
```

Which expands to (run individually when iterating):

```sh
pnpm -r build                # core dist + desktop web assets (typecheck/lint need core's dist)
pnpm check:ipc               # Rust ↔ TS IPC contract drift guard (scripts/check-ipc.mjs)
pnpm check:privacy           # privacy tripwires (scripts/check-privacy.mjs)
pnpm lint                    # eslint (type-checked, strict)
pnpm format:check            # prettier
pnpm typecheck               # tsc across packages
pnpm -r test                 # vitest: packages/core + apps/desktop
cd apps/desktop/src-tauri    # (pnpm check:rust runs these four)
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI (`.github/workflows/ci.yml`) runs exactly these.

### Real-model STT test (optional, ignored by default)

```sh
curl -LO https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
OPENFLOW_TEST_MODEL=$PWD/ggml-tiny.en.bin cargo test -- --ignored
```

## macOS permissions in development

TCC grants attach to the **process that asks**. Under `pnpm dev` that is your terminal
(or editor), not an app bundle:

- **Microphone** — the prompt appears on first recording; grant it to your terminal. If you
  denied it once: System Settings → Privacy & Security → Microphone → enable your terminal.
- **Accessibility** — needed for the ⌘V paste and the ⌘C selection capture. Add your terminal
  under System Settings → Privacy & Security → Accessibility. Without it, OpenFlow still works
  but copies results to the clipboard instead of pasting.
- After `pnpm tauri build`, the bundled `OpenFlow.app` asks for its own grants on first run.
- Hotkeys need no extra permission (Carbon `RegisterEventHotKey`).

## Manual test checklist

Automated tests cover the logic; these flows need a human:

1. Onboarding completes; both permission steps reflect reality; model downloads with progress.
2. Hold `⌥Space` in Notes → speak → release → text pasted; previous clipboard restored.
3. Quick-tap `⌥Space` → keep talking hands-free → tap again → text pasted.
4. Tray: switch mode → dictate → output style changes; _Copy Last Result_ works.
5. With Ollama running (`ollama pull qwen2.5:3b`): Standard mode produces polished text; kill
   Ollama mid-flight → dictation still inserts rules-cleaned text + amber notice.
6. Select a flawed sentence in TextEdit / Safari / Slack, tap `⌥⇧P` → selection replaced with
   the polished text; no recording HUD appears.
7. Tap `⌥⇧P`, then click elsewhere in the same field while "Polishing selection…" shows → the
   result is inserted at the caret instead of replacing (nothing is lost).
8. Settings → Dictation: toggle _Polish with AI_ off → dictate with Ollama running →
   no request appears in `ollama serve` logs; toggle from the tray and confirm Settings follows.
9. Revoke Accessibility → dictate → clipboard-only notice appears; `⌥⇧P` shows the grant
   hint instead.
10. Settings: change hotkey to a taken combo (e.g. `Cmd+Space`) → clear error, old hotkey kept.
11. Models tab: create a second profile, switch the active radio between them mid-session →
    next polish uses the newly selected profile; _Show in Finder_ opens `<app-data>/profiles/`.
12. Put another app (Safari, an editor) in macOS full-screen, then hold `⌥Space` → the HUD pill
    shows over the full-screen app and the text still pastes into that app.
13. Settings → Snippets: add `my email → me@example.com` (inline) → dictate "send it to my email"
    → the address is inserted in place. Add a "spoken alone" snippet and confirm it expands only
    when said by itself, not mid-sentence.
14. Settings → Dictation → Polish: add the "Concise" template, assign a shortcut, select a wordy
    sentence in any app, press it → the selection is rewritten; the HUD reads "Concise…". Assign
    a hotkey already taken by dictation → clear error, the change is rolled back.
15. Dictate a sentence naming a product (e.g. "we use TanStack and DeepSeek") → open Dictionary →
    both appear under "Noticed while you spoke"; click one to add it ("kept as-is"), dismiss the
    other → it disappears for the session. Insights → the words/pace/streak update after dictating.
16. Tray → switch to Code mode → hold `⌥Space`, say "get user by id" → `getUserById` is inserted;
    say "constant max retries" → `MAX_RETRIES`. Toggle Show in Dock on → a Dock icon appears and
    survives closing Settings; off → menu-bar only.

## Repository layout

```
packages/core/            TS contract (mirrors Rust serde structs) + pure utils + tests
apps/desktop/src/         React: main settings app (index.html) + HUD (hud.html)
apps/desktop/src-tauri/   Rust core — see docs/ARCHITECTURE.md §2 for the module map
scripts/                  check-ipc.mjs + check-privacy.mjs (CI guards), generate-icon.mjs, release.sh
```

Keep the IPC contract in sync: any change to a serde struct in
`src-tauri/src/settings.rs` / `pipeline.rs` / `models.rs` must be mirrored in
`packages/core/src/types.ts` (field names are camelCase on the wire).
`pnpm check:ipc` catches name-level drift; the full rules and add-a-command
checklist live in `docs/engineering/ipc-contract-conventions.md`.

## Releasing (maintainers)

```sh
./scripts/release.sh         # checks, builds the DMG, prints artifact paths
```

Tagging `v*` runs `.github/workflows/release.yml`, which builds the DMG on a macOS runner and
attaches it to a draft GitHub release. Builds are currently **unsigned/un-notarized** — users
right-click → Open on first launch. Signing + notarization needs an Apple Developer ID and is
tracked in the roadmap.
