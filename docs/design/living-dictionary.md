# A dictionary that learns (with your permission)

Status: **shipped** (Approach B — in-RAM, session-only). Began as a brainstorm sketch; this doc
now records what was built. One refinement emerged while building: a suggested term that whisper
already spells correctly has no `from → to` correction, so accepting one writes a `from == to`
**vocabulary entry** (preserve this spelling — it still biases whisper and the LLM), rendered in
the list as the bare term with a "kept as-is" tag. The detector ships internal-caps only;
digit-bearing and recurring-correction signals remain follow-ups.

## Why

The personal dictionary is one of the most useful features and one of the most underused,
because every entry is manual. Nobody adds "Kubernetes" or "Redisson" until whisper has mangled
it five times and they have noticed, opened settings, found the Dictionary tab, and typed it in.
The terms a user repeats are sitting right there in their own usage — but discovering them must
**never** mean recording or storing what they say. That tension is the whole design.

## Idea

Surface gentle, accept-or-ignore suggestions for dictionary entries, derived only from signals
OpenFlow can read without keeping a transcript. Entries that came from a suggestion wear a small
badge (a ✨ or "suggested" tag) so the list stays honest about where each one came from.

Two deterministic signals, no model, no corpus:

1. **Internal-caps / mixed tokens.** When whisper writes a token with internal capitals or digits
   — `TanStack`, `DeepSeek`, `v1.5`, `OpenClaw` — it is almost always a proper noun or product
   name the user will want preserved. These are cheap to spot in the cleaned text.
2. **In-session recurrence.** A token (or a phrase the user keeps correcting via the existing
   dictionary) that recurs several times within one app session is a candidate worth offering.

## The constraint that shapes everything

OpenFlow persists **nothing** — no audio, no transcripts, no history. "Learning" therefore cannot
mean accumulating a corpus on disk. The design has to produce suggestions from _ephemeral_ state.

| Approach               | Where candidates live                           | Disk?        | Verdict                             |
| ---------------------- | ----------------------------------------------- | ------------ | ----------------------------------- |
| A — last-result only   | the in-memory last transcript                   | none         | safe but thin                       |
| **B — session counts** | a small in-RAM count of _candidate tokens only_ | **none**     | **recommended**                     |
| C — persisted counts   | a counts file on disk                           | yes (opt-in) | crosses the line; gate like history |

**Recommend B.** During a session, keep a tiny in-memory table of candidate terms and how often
each appeared — _candidate tokens only, never full text_ — and surface the top few as
suggestions. It dies on quit. Accepting a suggestion writes a normal dictionary entry (persisted
exactly as a hand-typed one is today); the ✨ badge records that it began as a suggestion.
Nothing else touches disk, and nothing is ever transmitted.

This mirrors a principle OpenFlow already lives by: a hint computed from local state is fine;
exporting the state that produced it is not.

## UX

Suggestions should be quiet and dismissible — a tool that nags about its dictionary is worse than
one that stays silent. Candidate surfaces, lightest to heaviest:

```text
Dictionary                                                   [ + Add ]

  Suggested for you  (seen this session — nothing was saved)
  ┌──────────────────────────────────────────────────────────┐
  │ ✨ Redisson      seen 4×        [ Add ]   [ Dismiss ]      │
  │ ✨ TanStack      mixed caps     [ Add ]   [ Dismiss ]      │
  └──────────────────────────────────────────────────────────┘

  Your entries
  open flow      → OpenFlow
  Redisson       → Redisson        ✨ added from a suggestion
```

Alternative, even lighter: a single inline chip on the **Last result** card — "Add _Redisson_ to
your dictionary?" — right where the user just saw the word appear. No separate list to visit.

## How it fits

A small `suggestions` module holding the in-RAM candidate table, fed by `text.rs` after each
clean (it already produces the final text; it just also notes candidate tokens). The settings UI
reads the table over a new `list_dictionary_suggestions` command; **accepting** is just
`save_settings` with one more `DictionaryEntry`. No persistence layer, no new file, no schema
change beyond an optional `source: "suggested" | "manual"` tag on `DictionaryEntry` for the
badge.

```rust
// suggestions.rs (in-memory only; cleared on quit)
struct Candidates { counts: HashMap<String, u32> }   // candidate tokens, not transcripts
impl Candidates { fn observe(&mut self, cleaned: &str); fn top(&self, n: usize) -> Vec<Suggestion>; }
```

## Privacy fit

This is the doc where privacy _is_ the feature, and the architecture has to earn the claim:

- Candidate counts live in RAM, are bounded, and are erased on quit. No corpus, no file.
- Only **candidate tokens** are ever held — not sentences, not the surrounding text.
- Accepting writes one ordinary dictionary entry; declining leaves no trace.
- Zero network. The "OpenFlow noticed you say X often" line is computed entirely on-device, the
  same way local hints already are.

If we ever want persistent, cross-session suggestions (e.g. "you've said this across many days"),
that requires storing counts and must ride the **same opt-in gate as local history** — never on
by default. Approach B needs none of that.

## Open questions

- Aggressiveness and noise control: a minimum count, a per-session suggestion cap, and a
  permanent "don't suggest this" list so dismissals stick within the session.
- Where suggestions live — Last-result chip (discoverable, in-flow) vs. Dictionary banner
  (centralised). Possibly both, same data.
- Do auto-detected code identifiers feed [code-dictation](code-dictation.md)? They are the same
  internal-caps signal; the two features share the detector.
- The badge wording must make provenance obvious without implying anything was stored.
