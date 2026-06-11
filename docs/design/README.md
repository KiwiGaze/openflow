# Design explorations — making dictation do more, still local

Status: brainstorm. Written against the shipped app (`main`). These are ideas we have been
kicking around for where OpenFlow goes after the core loop — dictate, rewrite, polish — settled.
Each is a sketch meant to provoke a decision, not a finished spec.

## The frame

The core product works: hold a key, speak, get clean text in the active app; select text and
fix or rewrite it. What it _doesn't_ yet do is the long tail of small, daily conveniences that
make a tool feel like it disappears into the workflow. This package collects six of those, each
chosen because it adds capability **without** spending the things that make OpenFlow worth using.

Three principles held every idea to account:

1. **Local by default, always verifiable.** Nothing leaves the Mac unless the user wired up their
   own endpoint. No new feature may add a connection that dictation alone would not. Where a
   feature is _about_ data (insights, a learning dictionary), the privacy design _is_ the design.
2. **Persist nothing the user didn't ask for.** "Nothing is stored" is the default story.
   Anything that writes to disk is opt-in, bounded to aggregates or settings, and resettable.
3. **Don't add a second switchable concept, and keep the critical path fast.** Modes are the one
   switchable thing. New ideas attach to existing surfaces (the dictionary, the refine call, the
   rules cleanup) rather than inventing parallel ones. Deterministic and offline beats an LLM
   round-trip wherever it can.

## The six ideas

| Idea                                      | One line                                                    | Touches                            | Network                    | Persists                        |
| ----------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | -------------------------- | ------------------------------- |
| [Snippets](snippets.md)                   | `trigger → expansion`; say a little, write a lot            | `text.rs`, settings                | none                       | settings only                   |
| [Quick transforms](quick-transforms.md)   | Polish generalised into a shelf of one-tap text tools       | `pipeline.rs`, `modes.rs`          | existing refine call       | settings only                   |
| [Living dictionary](living-dictionary.md) | suggest vocabulary from your own usage, no corpus           | `text.rs`, new in-RAM module       | none                       | settings only (on accept)       |
| [Local insights](local-insights.md)       | a private mirror of your usage that never uploads           | new `stats` module, `pipeline.rs`  | **none, ever**             | session-only; opt-in aggregates |
| [Code dictation](code-dictation.md)       | speak identifiers, get `camelCase`/`snake_case`             | `text.rs`, `modes.rs`              | tier 1 none; tier 2 refine | none new                        |
| [Ambient polish](ambient-polish.md)       | sounds, focus ducking, notifications, presence, UI language | `audio.rs`, `output.rs`, `main.rs` | none                       | settings only                   |

## Status — what shipped

All six landed on this branch in highest-leverage order, each in its own commit with the full
CI suite green (lint, format, typecheck, TS tests, `cargo fmt`/`clippy`/`test`). Every "smart"
surface is computed from local state only; nothing new leaves the machine or is persisted by
default.

1. **Snippets** ✓ — `trigger → expansion` on the dictation path, after the LLM so expansions
   stay verbatim; single-pass matcher shared with the dictionary (no cascading). Snippets tab.
2. **Quick transforms** ✓ — `polish()` generalized into `refine_selection`; named, hotkey-bound
   instructions on a selection with templates and inline editing. The deterministic **register**
   idea remains a noted follow-up.
3. **Local insights** ✓ — session-only, in-memory aggregates (words, pace, dictations, AI share,
   top modes); no file, no network. Opt-in `stats.json` for cross-session streaks is the
   follow-up.
4. **Living dictionary** ✓ — in-RAM internal-caps detector suggests product/proper names;
   accepting writes a `from == to` vocabulary entry ("kept as-is"). Digit/recurring-correction
   signals are follow-ups.
5. **Code dictation** ✓ (first slice) — a built-in **Code** mode turns each utterance into one
   identifier (camelCase default; leading keyword picks snake/pascal/constant/kebab). The symbol
   grammar (spoken "open paren") and Tier-2 LLM formatting are deferred — the spacing problem
   needs its own pass.
6. **Ambient polish** ◐ (partial) — **Show in Dock** shipped (activation-policy toggle). Sounds,
   focus ducking, completion notifications, and UI localization are deliberately deferred: each
   needs new infrastructure (audio playback, the notification plugin, an audio-session API, or an
   i18n track) that warrants its own focused change.

## Threads that connect them

These are not six isolated features; several share machinery, which is part of why they are worth
doing together:

- The **internal-caps detector** powers both the living dictionary's suggestions and code
  dictation's identifier survival — build it once.
- The **whole-phrase matcher** behind the dictionary is reused by snippets.
- The **selection-refine chain** behind Polish is reused verbatim by quick transforms.
- **Per-app activation** (already on the wider roadmap) is the cleanest trigger for code
  dictation and the source of the bucketed app categories in insights.
- Every "smart" surface — a suggestion, an insight, a tip — is **computed from local state only**
  and never from a stored corpus. That single rule is what lets all of this stay private.

## Explicitly out

Anything that needs a server or reads the screen: team sync of snippets or dictionaries, cloud
dashboards, deep editor/IDE scraping, a template or transform marketplace, and any always-
listening trigger. If an idea here only works by sending data off the machine, it does not belong
in OpenFlow — and none of the six requires it.
