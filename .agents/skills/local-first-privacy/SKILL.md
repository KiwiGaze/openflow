---
name: local-first-privacy
description: Verify any OpenFlow change against the privacy invariants — audio never persisted, no telemetry, cloud strictly opt-in + BYO-key + consent-gated, network code confined to three modules. Use this for every change that touches audio, networking, persistence, logging, history, profiles, consent, or adds any dependency; privacy is the product and violations are unmergeable.
---

# Local-first privacy

## Purpose

Privacy here is architectural, not contractual: by default audio never
leaves the Mac, nothing is written that the user didn't opt into, and there
is no telemetry code at all. Every change must keep those statements true —
they are the product's reason to exist, and reviewers reject violations
without discussion.

## Files to inspect first

- `PRIVACY.md` — the complete data-flow statement and endpoint allowlist
  (the promise to users; authoritative).
- `docs/engineering/architecture-boundaries.md` §"Privacy boundaries".
- `scripts/check-privacy.mjs` — the mechanical tripwires and their
  allowlist.
- For consent flows: `apps/desktop/src/app/components/SttEngines.tsx`
  (the gate UI) and `pipeline.rs`'s consent check before any upload.

## Procedure

1. Classify the data your change touches: audio, transcript/selection text,
   counters, settings, keys. Then check the storage rule: audio — memory
   only, ever; text — only behind `historyEnabled`; counters — in-memory,
   content-free, reset on quit; keys — 0600 profile files, bearer auth to
   the profile's own URL only.
2. Network changes: code stays inside `llm.rs`, `models.rs`, or
   `cloud_stt.rs`. A genuinely new network surface means updating
   `PRIVACY.md`, the `check-privacy.mjs` allowlist, and the PR description
   in the same PR — expect the review to center on it.
3. Anything that sends user content to a configured endpoint must sit behind
   the existing gates: an explicit profile + (for audio) the per-profile
   consent in `confirmedSttProfiles`. Editing a profile's endpoint or key
   revokes its consent — preserve that behavior.
4. Logging: never log transcript text, selections, audio, or keys; provider
   bodies on the audio path are logged status-only.
5. Run `pnpm check:privacy` and read every line it flags.

## Rules

- No telemetry, analytics, or crash reporting — including "anonymous" and
  "temporary". The manifests are scanned for known SDK names.
- No default-on network call may carry user content; the only unprompted
  traffic is the user-initiated model download.
- The webviews do no network I/O — every byte goes through Rust where the
  gates live.
- New persistence of any kind is opt-in, named in the PR, documented in
  PRIVACY.md.
- Runtime env vars are a design change (invisible behavior switches) —
  don't add them.

## Commands

```sh
pnpm check:privacy   # tripwires: HTTP confinement, webview I/O, telemetry deps
pnpm check:all       # the full gate
```

## Checklist

- [ ] Each data class touched has its storage rule upheld
- [ ] Network code confined (or allowlist + PRIVACY.md updated, with reason)
- [ ] Consent gates intact, including revoke-on-endpoint-change
- [ ] No content in logs; no new persistence without opt-in
- [ ] `pnpm check:privacy` green

## Common mistakes

- Caching audio or transcripts to disk "temporarily" for debugging — the
  invariant is absolute, and a `#[cfg(debug_assertions)]` exception still
  ships a footgun.
- Turning a session counter into a timestamped log (a counter is a number;
  a log is surveillance).
- Logging a cloud provider's error body on the audio path — it can echo
  request content.
- Reusing a deleted profile's id and inheriting its old consent (the store
  drops consent on delete; keep it that way).

## Expected output

A change where every byte's path is accountable: gates intact,
`check:privacy` green, PRIVACY.md still true word-for-word.
