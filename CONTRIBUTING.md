# Contributing to OpenFlow

Thanks for helping build a private, free alternative to cloud dictation. All kinds of
contributions are welcome: bug reports, docs, code, and real-world dictation feedback
(accents, languages, flaky apps).

## Ground rules

- Be kind. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
- Privacy is the product. PRs that add telemetry, default-on network calls, or audio
  persistence will not be merged. Cloud anything must be opt-in and BYO-key.
- Keep the dictation critical path fast and dependency-light.

## Getting started

1. Fork and clone; `pnpm install`; `pnpm dev`. Full setup: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).
2. Find an issue labeled `good first issue`, or open an issue to discuss anything substantial
   before building it.

## Pull requests

- Branch from `main`. Keep PRs focused — one change per PR.
- Make sure everything passes locally; CI runs the same commands:
  ```sh
  pnpm lint && pnpm format:check && pnpm typecheck && pnpm -r test
  cd apps/desktop/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
  ```
- Add tests for logic changes (text processing, settings, IPC shapes). UI-only changes should
  include before/after screenshots.
- If you touch a serde struct that crosses IPC, update its mirror in
  `packages/core/src/types.ts` in the same PR.
- Commit messages: conventional-ish (`feat:`, `fix:`, `docs:`, `chore:`) with a scope when it
  helps (`feat(stt): …`).

## Project conventions

- **Rust:** `rustfmt` defaults; clippy clean with `-D warnings`; no `unwrap()` outside tests —
  poisoned-lock `expect()`s are the accepted exception; errors become user-readable `AppError`s.
- **TypeScript:** strict; no `any`; explicit return types on exported functions.
- **Comments** explain _why_ (invariants, OS quirks), not _what_.

## Reporting bugs

Use the bug template. Always include: macOS version, chip (M1/M2/…), the app you were dictating
into, which speech model, and whether an AI provider was configured. Logs live at
`~/Library/Logs/app.openflow.desktop/`.

## Security

Found a vulnerability? Please do not open a public issue — email the maintainers (see
repository profile) and allow a reasonable window for a fix.
