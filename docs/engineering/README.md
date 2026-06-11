# Engineering docs

The conventions that keep this repo modifiable without guesswork — by new
contributors and by AI agents alike.

- [monorepo-conventions.md](monorepo-conventions.md) — layout, scripts,
  tests, logging/errors, env vars, docs placement, naming, comments, when to
  split files.
- [architecture-boundaries.md](architecture-boundaries.md) — Rust/React/core
  ownership, threading model, the constraints that don't show up in a diff,
  macOS-only assumptions, privacy boundaries.
- [ipc-contract-conventions.md](ipc-contract-conventions.md) — the
  hand-mirrored IPC contract: where it lives, naming/shape rules, add-a-
  command checklist, what `pnpm check:ipc` enforces.
- [review-checklist.md](review-checklist.md) — PR blockers, required
  commands, per-area review rules.
- [research/monorepo-best-practices.md](research/monorepo-best-practices.md)
  — the sourced research behind these choices, including what was
  deliberately skipped.

Related: repo-local agent skills live in `.agents/skills/`; the system
design rationale is `docs/ARCHITECTURE.md`; privacy promises are
`PRIVACY.md`.
