# Insights without surveillance

Status: **Tier 1 shipped** (session-only, in-memory). Began as a brainstorm sketch; the
no-disk version is built. Tier 2 (opt-in `stats.json` for cross-session streaks) remains a
deliberate follow-up.

## Why

People genuinely enjoy seeing their own numbers: how many words they dictated, how fast they
speak, a streak that rewards showing up. It is a real retention and delight feature — and every
tool that offers it does so by shipping your usage to a server, because the dashboard _is_ the
business model.

But the computation is entirely local. The only reason "insights" usually means "telemetry" is
commercial, not technical. Velata is in the rare position of being able to hand users a mirror
of their own usage that, by construction, never leaves the Mac. That contrast is itself the
feature.

## The line we will not cross — and the one we can

Velata persists nothing and sends nothing. Insights has to respect that. The unlock is a
distinction:

> An **aggregate** is not **content**. "You have dictated 22,000 words" is a number. Your words
> are not in it.

Counts, sums, and dates are defensible to compute and (if opt-in) to store, because you cannot
reconstruct a single sentence from them. Storing the _text_ would be history (a separate, opt-in
thing); storing _"how many words, total"_ is a tally.

| Data                                      | Is it content?     | Allowed                  |
| ----------------------------------------- | ------------------ | ------------------------ |
| total words inserted, total recording ms  | no — integers      | yes                      |
| per-mode counts, per-output-target counts | no — integers      | yes                      |
| daily activity (a date had ≥1 dictation)  | no — a date bitmap | yes (needed for streaks) |
| any transcript, any phrase, any audio     | **yes**            | **never**                |
| any timestamp tied to specific content    | yes-ish            | no                       |

## Idea

A local-only **Insights** view: words dictated, an average speaking pace (words ÷ recording
seconds), the mode you use most, where your dictation lands (which app categories), and a streak.
All of it derived from counters, all of it on this Mac, none of it uploaded — ever.

```text
Insights                          Computed and stored only on this Mac · no account, no upload

   22,295            72 wpm              ✦ 3-day streak   (longest 44)
   words dictated    your pace          ▓▓░▓▓▓░▓▓▓▓▓░▓▓  ← local calendar

   Where it goes                         Most-used
   ▓▓▓▓▓▓▓░░░  editors      43%          1. Standard   58%
   ▓▓▓▓░░░░░░  documents    30%          2. Email      24%
   ▓▓░░░░░░░░  messages     27%          3. Notes      18%

   [ Reset stats ]                          Verify with Little Snitch: zero connections.
```

## Two tiers, ship the cheap one first

Most of this needs no disk at all; only the streak inherently spans sessions.

- **Tier 1 — session-only (no file, no toggle, no policy change).** Keep the counters in memory
  for the running session and show them. Reset on quit. This requires _no_ persistence decision,
  so it can ship without touching the "nothing is stored" promise. It already delivers words,
  pace, per-mode and per-target breakdowns for "today."
- **Tier 2 — opt-in `stats.json` (aggregates only).** A small file holding _only_ the integers
  and the date bitmap above, written atomically like `settings.json`, **off by default**.
  Turning it on is what enables the multi-day streak and lifetime totals. The toggle's copy is
  explicit: "Stores counts and dates — never your words or audio — only on this Mac."

Recommend building Tier 1 first: it is the larger share of the value at none of the persistence
risk, and it lets us see whether the surface is worth the Tier-2 file at all.

## How it fits

A `stats` module with an in-memory counter set that `pipeline.rs` increments at the end of a
successful insert (it already knows the word count, the mode, recording duration, and the insert
outcome). A `get_insights` command returns the snapshot. Per-app-category counts reuse the
frontmost-app bundle id that per-app behavior already needs — bucketed into coarse categories
(editor / document / messaging / other), never logging _which_ app, only the bucket.

```rust
// stats.rs (Tier 1: in-memory; Tier 2: optional aggregates file)
struct Stats { words: u64, record_ms: u64, by_mode: HashMap<String,u64>, by_bucket: HashMap<Bucket,u64> }
```

Tier 2 adds: load on launch, flush throttled, a `clear_stats` command, and the `appStatsEnabled`
settings flag. No new IPC events — the Insights view pulls on open.

## Privacy fit

The point of the doc. The design choices that make it Velata-native rather than a tracked
dashboard wearing a privacy hat:

- **Opt-in, and off by default** for anything written to disk; Tier 1 writes nothing.
- **Aggregates only.** No text, no audio, no per-event content timestamps — enforced by the
  schema, which has nowhere to put them.
- **Verifiable**: the view states the stats are tallied in memory and never written or uploaded,
  and is explicit that the network is touched only on an opt-in (a cloud profile or a model
  download) — so the privacy claim is scoped and testable rather than an over-broad "zero
  connections."
- App attribution is **bucketed**, so "where it goes" never becomes "a log of every app you used."

## Open questions

- Is Tier 2 worth the complexity, or is session-only enough? Streaks are the main thing that
  needs the file; gauge demand before adding it.
- "Your voice" — surfacing the words you correct most — is genuinely useful (it feeds
  [living-dictionary](living-dictionary.md)) but edges toward content. Keep it to counts of
  _dictionary hits_, not raw phrases, or leave it out.
- Pace (wpm) needs recording seconds; make sure that is wall-clock speech, not including
  transcription/refine time, or the number lies.
- A "share" affordance is tempting and dangerous — any share turns local stats into an outbound
  payload. If offered at all, it must be an explicit, user-built image, never automatic.
