# @velata/core

The shared TypeScript surface of Velata: the hand-written mirror of the
Rust IPC contract plus pure utilities used by the webviews.

- `src/types.ts` — the IPC mirror: every serde struct that crosses Tauri IPC,
  the `COMMANDS`/`EVENTS` name maps, and shared wire constants
  (`CLOUD_STT_PREFIX`, profile versions). Changing a Rust IPC struct means
  changing this file in the same PR; `pnpm check:ipc` enforces the name-level
  half of that. Rules: [docs/engineering/ipc-contract-conventions.md](../../docs/engineering/ipc-contract-conventions.md).
- Everything else (`hotkey`, `format`, `validate`, `diff`, `presets`,
  `languages`, `dictionaryio`) — pure, dependency-free functions with colocated
  `*.test.ts`.

Ground rules: no `@tauri-apps/*`, no React, no I/O — if it isn't pure or
isn't shared, it belongs in `apps/desktop`. The package is workspace-internal
(`private: true`) and consumed from its built `dist/`, so run `pnpm -r build`
before `typecheck`/lint from a cold clone (or just use `pnpm check`).
