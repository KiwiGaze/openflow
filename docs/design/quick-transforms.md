# Quick transforms — Polish, generalized

Status: exploration. A brainstorm sketch, not a committed spec.

## Why

OpenFlow has two ways to edit a selection:

- **Polish** (`⌥⇧P`, tap) applies one fixed instruction — _fix grammar, spelling, clarity_ — and
  is fast and predictable.
- **Rewrite** (`⌥⇧Space`, hold) applies whatever you say — "make it shorter", "translate to
  German" — and is flexible but needs a spoken instruction every single time.

The gap is the edit you make _all day_ that is neither grammar nor ad-hoc: "make this concise,"
"turn this into bullets," "make it friendlier." With Polish you can't (it only fixes grammar);
with Rewrite you must re-speak the same instruction on every use. There is no way to _save_ the
instruction you keep repeating and fire it with one key.

## Idea

Let the user define a few **transforms** — a name, a prompt, and an optional hotkey — and apply
one to the selection with a single tap, no voice. Polish stops being a special case and becomes
the first, built-in member of the set.

The framing that keeps this from becoming a new concept: **a transform is a saved Rewrite
instruction with a hotkey.** Nothing new in the pipeline — the selection-capture → refine →
insert chain is exactly today's, only the instruction comes from a saved transform instead of
`DEFAULT_REFINE_INSTRUCTION` or a spoken phrase.

```
Modes      = how DICTATION becomes written text        (Standard, Email, Notes, Literal)
Transforms = how an EXISTING SELECTION is rewritten    (Polish, Concise, Bullets, …)
  Polish   = the default transform (fixed instruction)
  Rewrite  = an ad-hoc transform   (spoken instruction)
```

One axis (selection refinement), several saved instructions — not a second switchable thing.

## UX

A shelf under **Refine**, next to Polish. A few ship as starting points; "Create your own" is a
blank prompt plus a hotkey picker.

```
Transforms                                            [ + Create your own ]

  ⌥⇧P   Polish        Fix grammar, spelling, clarity.        (built-in)
  ⌥1    Concise       Tighten wording; keep meaning & tone.
  ⌥2    Bullet points Restructure into short bullets.
  —     Friendlier    Warmer, more casual register.

Edit transform
  Name         Concise
  Hotkey       ⌥1                                    [ Change ]
  Instruction  ┌──────────────────────────────────────────────┐
               │ Tighten the wording. Keep the meaning, tone,  │
               │ and language. Do not add new information.      │
               └──────────────────────────────────────────────┘
  Preview ▸  "i was thinking maybe we could possibly…" → "Let's…"
  [ Delete ]                                            [ Save ]
```

**Preview** runs the instruction against a short sample using the active AI profile (rules
fallback when offline), so an author sees what a transform does before trusting it on real text.

## How it fits

Almost free, because the machinery exists. `pipeline.rs` already runs the Polish flow
(`Job::PolishSelection`): capture selection → refine with an instruction → insert. A transform
is the same job parameterised by `transformId`:

```rust
// pipeline.rs
Job::Transform(String)      // the transform id; instruction looked up at job start

// the refine call already takes an instruction string — pass the transform's,
// reusing selection_system_prompt() and the replace → caret → clipboard insert chain verbatim
```

Data model: `transforms: Vec<Transform> { id, name, instruction, hotkey }` in `settings.json`
(camelCase mirror). Hotkeys register through the same path as the other three and join the same
**N-way pairwise-distinct** check, with the same small cap (≤ 5) the per-mode-hotkey idea uses,
so the keyboard surface stays sane. `commands.rs` gains `start_transform(id)`; everything else
is reuse.

## A lighter cousin: deterministic register

Not every tone shift needs an LLM round-trip. A **register** is a cheap, offline formatting
adjustment — capitalization on/off, punctuation density, exclamation level — applied in
`text.rs` rules cleanup:

```
formal       → Caps + full punctuation.     "Hey, are you free tomorrow?"
casual       → Caps + light punctuation.     "Hey are you free tomorrow"
very casual  → no caps + light punctuation.  "hey are you free tomorrow"
```

This is deterministic, instant, network-free, and useful to people who never configure an AI
profile. It could be a per-mode hint or a one-tap transform of its own. Worth prototyping
precisely because it gives "tone control" to the offline, zero-config user — the person the
LLM-based transforms can't serve.

## Privacy fit

Identical to Polish and Rewrite: a transform needs an AI profile, sends **text only** to the
user's own endpoint (BYO key), and opens no new network surface — it is the existing refine call
with a saved instruction. The deterministic register adds no network at all.

## Open questions

- Hotkey budget across Dictation + Rewrite + Polish + Transforms — enforce a hard cap and show
  conflicts up front.
- Should a transform also be runnable on **fresh dictation** (chain after cleanup), or strictly
  on a selection? Strictly-selection is simpler and matches Polish; revisit if asked.
- Sharing: transforms are small JSON; "Show in Finder" + a docs gallery (files, not a
  marketplace) matches how profiles and modes are meant to travel.
- How many ship by default — a tight, opinionated few (Concise, Bullets, Friendlier) beats a
  long list nobody reads.
