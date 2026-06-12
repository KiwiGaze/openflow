# Review checklist

What a reviewer (human or agent) checks on an Velata PR, ordered by blast
radius. `.coderabbit.yaml` encodes the same rules for automated review.

## Blockers — reject without further discussion

- Telemetry, analytics, crash reporting, or any default-on network call.
- Audio persisted anywhere (disk, event payload, log, history).
- Transcript/selection text in logs, or persisted outside the opt-in history.
- Cloud anything that is not opt-in + BYO-key + consent-gated.
- A new outbound endpoint not listed in PRIVACY.md.
- IPC struct changed on one side of the mirror only.
- `unwrap()` outside tests (poisoned-lock `expect()` is the accepted
  exception); clippy or any `check:*` script failing.
- Dictation output that can vanish silently on failure — worst case must be
  "text on the clipboard + HUD notice".

## Required commands (all must pass)

```sh
pnpm check        # build, check:ipc, check:privacy, lint, format, typecheck, TS tests
pnpm check:rust   # frontend build, cargo fmt --check, clippy -D warnings, cargo test
```

`pnpm check:all` runs both. CI runs the same lists; a PR that needs a command
CI doesn't run is changing the quality gate — flag it.

## Rust review

- New OS/compute work lives in Rust, not the webview; new webview logic is
  presentation-only ([architecture-boundaries.md](architecture-boundaries.md)).
- Check the diff against the seven "constraints an agent can break" — above
  all: generation checks around any new pipeline state write, and
  `spawn_blocking` for anything that reaches the output worker from a
  command.
- Errors: new failure paths produce an `AppError` with a HUD-ready message
  (sentence case, actionable, no module paths). No `let _ =` on a fallible
  call whose failure the user would care about.
- Threading: no new `async` in the audio/output paths; cross-thread talk via
  channels; no lock held across an `.await`.
- New file-backed stores copy the profiles pattern: `safe_id` validation,
  0600, atomic tmp+rename, corrupt files skipped never deleted.
- Doc comments on new public items and every new IPC command.

## TypeScript / React review

- No duplicated backend logic; no direct network I/O; IPC only through
  `ipc.ts` wrappers.
- Strict types hold: no `any`, no `as` casts that paper over a design gap
  (a cast to a union from user/DOM input wants a type guard or a typed
  source instead).
- Shared values come from `@velata/core`, not re-declared locally
  (presets, versions, wire prefixes, validation).
- Hooks: effects clean up their listeners; no per-render `invoke` storms
  (bootstrap once + subscribe); dependency arrays honest.
- Components stay focused; extract only when the audit-documented seams say
  so ([monorepo-conventions.md](monorepo-conventions.md) §"When to split").

## IPC review

Follow [ipc-contract-conventions.md](ipc-contract-conventions.md). Fast
check: both sides in one diff, names follow the verb rules, `check:ipc`
green, wrappers added, doc comment on the command.

## Audio / STT review

- Audio buffers stay in memory and inside the pipeline; they may reach
  `cloud_stt.rs` only behind the consent check (profile present AND id in
  `confirmedSttProfiles`).
- Changes to `audio.rs`/`resample.rs` keep the dedicated-thread model and the
  RMS atomic; whisper stays in `spawn_blocking` with the context reused.
- Anything touching consent (`save_stt_profile` revoking on endpoint change,
  `delete_stt_profile` dropping consent) keeps those behaviors — they are
  product promises (08 §3).
- Latency matters: nothing new on the record→insert path that blocks, polls,
  or allocates per-sample.

## LLM / API-key review

- One client (`llm.rs`); providers are presets + profiles, never new code
  paths. Ollama-native API is for model listing only.
- Keys live in 0600 profile files, sent only as bearer auth to the profile's
  own `baseUrl`; never logged, never in events, never in settings.json.
- Prompts treat transcript/selection as data (injection-resistant framing);
  cloud responses on the audio path are logged status-only.
- Timeouts honored from the profile; failures degrade to rules-based cleanup
  with a notice, never to dropped output.

## Privacy review (any PR)

- `pnpm check:privacy` green; if its allowlist changed, PRIVACY.md changed
  too, and the PR says why.
- New persistence of any kind named in the PR description, with its consent
  story. "Nothing else is written" is the default state.
- New counters/aggregates are session-only and content-free.

## Comments, naming, file organization

- Comments say why (constraint, quirk, tradeoff); no restating code; stale
  comments fixed in the same diff that made them stale.
- Names are domain words; no `util`/`helper`/`manager`/`data`/`temp`.
- New files land in the existing structure; a new top-level directory or
  package needs prior agreement.

## HUD / settings UI review

- HUD window is never hidden/shown — content fades; the window was created
  once at startup ([architecture-boundaries.md](architecture-boundaries.md)
  §constraint 3). Same for the changes overlay; interactivity flips via
  `set_ignore_cursor_events`.
- HUD stays subtle: state display + level meter; it never grabs focus, never
  takes clicks while faded.
- Settings UI: optimistic updates roll back on save error (the `useSettings`
  pattern); UI-visible strings are sentence case and ESL-friendly; new
  settings appear in `Settings` (Rust + TS), get defaults for older
  `settings.json` files, and bump `SETTINGS_VERSION` when migration is
  needed.
- UI-only PRs include before/after screenshots (PR template).

## Over-engineering filter

Reject reviews and diffs that add: abstractions with one call site,
speculative configurability, defensive code for states the types already
forbid, indirection without a measurable win, or cosmetic renames. The repo
optimizes for "boring and explicit".
