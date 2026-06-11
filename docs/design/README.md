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

## Recommendation — what to do first

If we pick up only a few of these, do them in this order. The ranking weighs daily value against
cost and privacy risk; the cheap, pure-local, zero-policy-change wins come first.

1. **Snippets.** Highest value-per-effort. Pure-local, deterministic, rides the dictionary
   machinery and the settings blob, no persistence-policy question. People use expansion every
   day. Ship this first.
2. **Quick transforms.** Nearly free — it is the existing Polish job parameterised by a saved
   instruction — and it closes the real gap between "fix grammar" (Polish) and "say it every
   time" (Rewrite). Bundle the deterministic **register** idea with it to serve offline users.
3. **Ambient polish (sounds + Dock/presence first).** A bag of afternoons that makes the app feel
   finished. Do sounds, notifications, and the Dock toggle early; hold ducking until restore-on-
   all-exits is bulletproof; treat UI localization as its own track.
4. **Living dictionary (session-only, approach B).** High delight, but the in-RAM candidate
   model needs care to stay honest. Worth it; not first, because the value depends on getting the
   privacy framing exactly right.
5. **Local insights (Tier 1, session-only).** Lovely and on-brand — privacy as the feature — but
   the least _functional_ of the set. Ship the no-disk version; only add the opt-in `stats.json`
   for streaks if people actually ask.
6. **Code dictation.** Real value for a narrower audience, and it leans on per-app activation
   landing first. Strong follow-on once the casing grammar and the shared internal-caps detector
   from the living dictionary exist.

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
