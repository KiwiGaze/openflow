# @velata/desktop

The Velata app: a Rust core under `src-tauri/` and three small React
webviews under `src/`.

- `src/app/App.tsx` (`index.html`) — the settings window.
- `src/app/Hud.tsx` (`hud.html`) — the dictation HUD overlay.
- `src/app/Changes.tsx` (`changes.html`) — the "see changes" diff overlay.
- `src/app/ipc.ts` — the only place `invoke`/`listen` are called; typed
  wrappers over names from `@velata/core`.
- `src-tauri/src/` — everything that touches the OS: audio, whisper, LLM
  HTTP, paste, hotkeys, persistence. Module map:
  [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md); ownership rules:
  [docs/engineering/architecture-boundaries.md](../../docs/engineering/architecture-boundaries.md).

The webviews are pure UI — state lives in Rust and arrives via IPC. Don't add
business logic, network calls, or persistence here.

Run from the repo root: `pnpm dev` (tauri dev), `pnpm tauri build` (DMG).
Package-local scripts: `pnpm build` here builds **web assets only**
(`build:vite`, also used by `tauri.conf.json` as `beforeBuildCommand`);
`dev:vite` exists for `beforeDevCommand` — don't call it directly.
