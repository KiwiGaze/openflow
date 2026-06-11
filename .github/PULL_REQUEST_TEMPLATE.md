## What

<!-- One paragraph: what changes and why. Link the issue if one exists. -->

## How it was tested

- [ ] `pnpm check:all` passes (TS + Rust + IPC + privacy gates — the same list CI runs)
- [ ] Manually exercised the affected flow (see checklist in `docs/DEVELOPMENT.md`)

## Contract & privacy

<!-- Delete lines that don't apply. Reviewers check these against docs/engineering/review-checklist.md. -->

- [ ] IPC change: Rust struct/command and its mirror in `packages/core/src/types.ts` are in this same PR
- [ ] No new network calls — or PRIVACY.md and the `check-privacy.mjs` allowlist are updated and the consent story is described below
- [ ] Nothing new is persisted — or it's named below with its opt-in story
- [ ] UI change: before/after screenshots attached

## Notes

<!-- Anything a reviewer should look at first; tradeoffs; follow-ups. -->
