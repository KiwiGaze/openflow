# Quick transforms — Polish, generalized

> **Renamed (feat/UX-redesign).** This concept shipped and is current, but the names changed:
> a transform is now a **Prompt** (settings field `transforms` → `prompts`; the per-item
> `hotkey` field → `shortcut`), managed on the App's Transform page. **Polish** is the one
> built-in (id `"polish"`, default `⌥⇧P`). The same Prompt can also run automatically after
> dictation when selected as the **post-dictation transform** via the HUD circle
> (`postDictationTransformId`). Read "transform"/"hotkey" below as "Prompt"/"shortcut".

Status: **shipped** (settings v3). Began as a brainstorm sketch; this doc now records what was
built, with the original reasoning intact. Preview and a deterministic register (below) remain
follow-ups.

## Why

Velata has two ways to edit a selection:

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

```text
Modes      = how DICTATION becomes written text        (Standard, Email, Notes, Literal)
Transforms = how an EXISTING SELECTION is rewritten    (Polish, Concise, Bullets, …)
  Polish   = the default transform (fixed instruction)
  Rewrite  = an ad-hoc transform   (spoken instruction)
```

One axis (selection refinement), several saved instructions — not a second switchable thing.

## UX

A shelf under **Refine**, next to Polish. A few ship as starting points; "Create your own" is a
blank prompt plus a hotkey picker.

```text
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

## How it fits (as built)

Almost free, because the machinery existed. `polish()` was generalized into one private
`refine_selection(job, instruction, mode_id, hud_label)` that captures the selection and refines
it with a given instruction; `polish()` is now just `refine_selection(PolishSelection, "", …)`
and a transform is `refine_selection(Transform, t.instruction, …)`. The selection-capture →
`selection_system_prompt()` → insert chain is reused verbatim, so the no-fallback rule and the
generation/cancel contract are identical for all three selection jobs.

`Job::Transform` is a **payload-less** (Copy) variant — the enum stays Copy. The hotkey handler
captures only the transform **id**; `run_transform(id)` looks the instruction up in current
settings at trigger time, so editing a transform takes effect with no re-binding, and a transform
deleted between keypress and dispatch is a silent no-op. The transform's name rides in the
pipeline-state `message`, so the HUD reads "Concise…" rather than a generic label. An **empty
instruction is valid** — it falls back to the Polish default — which is why a freshly created
transform is usable immediately and is kept by `normalize()` (only blank-_name_ rows are dropped).

Data model: `transforms: Vec<Transform> { id, name, instruction, hotkey }` in `settings.json`
(camelCase mirror), schema **v3**. `shortcuts::apply` now registers the three fixed hotkeys plus
each **bound** transform (empty hotkey = unregistered draft) and runs an **all-pairs
pairwise-distinct** check across the whole set, rolling back cleanly on any failure.
`save_settings` re-registers only when an id↔hotkey **binding** changes, so name/instruction edits
(which save on every keystroke) don't churn the global shortcuts. No new IPC command — transforms
fire from their own global hotkey, never from the UI or tray (clicking the menu bar would change
the frontmost app). UI: a Transforms gallery in the Refine tab with one-click templates, inline
editing, and the shared `HotkeyRecorder`.

## A lighter cousin: deterministic register

Not every tone shift needs an LLM round-trip. A **register** is a cheap, offline formatting
adjustment — capitalization on/off, punctuation density, exclamation level — applied in
`text.rs` rules cleanup:

```text
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
