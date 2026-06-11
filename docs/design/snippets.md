# Snippets — say a little, write a lot

Status: **shipped** (settings v3). Began as a brainstorm sketch; this doc now records what was
built, with the original reasoning intact.

## Why

People repeat themselves. The same email address, the same calendar link, the same
"thanks, I'll take a look and get back to you" — typed or dictated again and again. Today
OpenFlow has no shorthand for this; the personal dictionary fixes _spelling_ ("open flow" →
"OpenFlow"), but it is the wrong tool for "expand four spoken words into a four-line block,"
because dictionary entries are meant to be short, whole-word, and one-to-one.

A dictation app is the natural home for expansion: the trigger surface (your voice) is already
open every time you want to drop in boilerplate. Users who reach for a separate text-expander
today are working around a gap we can close locally.

## Idea

A **snippet** is a `trigger → expansion` pair. When a trigger phrase appears in the cleaned
transcript, it is replaced by its expansion just before the text is inserted. Expansions can be
long and multi-line; triggers are short spoken phrases the user chooses.

```text
spoken:    "sign off and my email"
expansion: "sign off"  → "Best,\nYijiazhen"
           "my email"  → "yijiazhen@example.com"
inserted:  Best,
           Yijiazhen
           yijiazhen@example.com
```

It is deliberately close to the dictionary in mechanism — whole-phrase, case-insensitive match
— but separate in intent and UI, so neither concept gets muddy:

|           | Dictionary                                           | Snippets                             |
| --------- | ---------------------------------------------------- | ------------------------------------ |
| Intent    | Fix a word whisper mishears                          | Expand chosen shorthand into a block |
| Shape     | short → short, one-to-one                            | short phrase → long / multi-line     |
| Also does | biases whisper (`initial_prompt`), listed to the LLM | insertion-time expansion only        |
| Lives in  | `dictionary: DictionaryEntry[]`                      | `snippets: Snippet[]`                |

## UX

A new **Snippets** tab (or a section under Dictionary), one flat row per snippet, matching the
existing settings idiom — name states what it does, no nested descriptions.

```text
Snippets                                              [ + New snippet ]

  my email        → yijiazhen@example.com
  my calendar     → https://cal.example.com/yijiazhen
  sign off        → Best,⏎Yijiazhen
  triage reply    → Thanks for flagging this — I'll take a look and …

Edit snippet
  Trigger     my email
  Expansion   ┌────────────────────────────────────────────┐
              │ yijiazhen@example.com                      │
              │                                            │
              └────────────────────────────────────────────┘
              [ Delete ]                            [ Save ]
```

The hard question is **disambiguation**: when the user literally says "send it to my email,"
should "my email" expand? Three options, in increasing safety and friction:

1. **Inline, always** — any occurrence expands. Lowest friction, occasional surprise. This is
   what classic text-expanders accept.
2. **Whole-utterance only** — a snippet fires only when the trigger _is_ the whole dictation
   ("my email", said alone). No surprises mid-sentence; you must pause to expand.
3. **Reserved verb** — expansion requires a lead word, e.g. "insert my email" / "snippet my
   email." Explicit, but a new spoken command to learn.

Lean: **(1) inline by default, with a per-snippet "whole utterance only" checkbox** for the
ambiguous ones. Most triggers people pick ("triage reply", "my cal link") are not phrases they
say in normal prose; the few that are can opt into strict matching.

## How it fits (as built)

Pure, fast, and offline — `apply_snippets` lives in `text.rs` beside `apply_dictionary`. It is
applied on the **dictation path only**, on the _final_ text — after the LLM or rules pass — so
the expansion is verbatim and never reworded, and so the selection-edit jobs (Rewrite, Polish)
that operate on the user's existing text are untouched:

```text
dictation:  clean → (LLM | rules | literal) → apply_dictionary → apply_snippets → insert
rewrite/polish:  (no snippet expansion — editing existing text)
```

Placing expansion after the LLM (rather than the doc's original "after dictionary in
`process()`") is the one improvement found while building: it keeps multi-line/URL/email
expansions exactly as typed instead of letting the model reflow them, and confines the feature
to dictation.

```rust
// text.rs
pub struct Snippet { pub trigger: String, pub expansion: String, pub whole_utterance: bool }

pub fn apply_snippets(text: &str, snippets: &[Snippet]) -> String { … }
```

The matcher is a **single-pass, multi-phrase** replacement (`replace_phrases`) now shared with
the dictionary. It matches longest-trigger-first with per-edge word boundaries, and — crucially
— substitutes against the original text in one pass, so a replacement is never re-scanned. That
kills cascading: a short `cal` snippet can't fire inside the URL a `my cal` snippet just
produced. A `whole_utterance` snippet is checked first and replaces the entire dictation only on
an exact (trimmed, case-insensitive) match.

Data model: `snippets: Vec<Snippet>` in `settings.json` (camelCase mirror in
`packages/core/src/types.ts`), schema **v3**, default empty; blank triggers/expansions dropped in
`normalize()`. No IPC commands beyond the existing `save_settings` — snippets ride in the
settings blob like the dictionary does. UI: a **Snippets** tab beside Dictionary
(`SnippetsTab.tsx`), validated by `validateSnippet` in `@openflow/core`.

## Privacy fit

Perfect. Expansion is deterministic string replacement on the user's own machine; no LLM, no
network, nothing new persisted beyond the snippet text the user typed into `settings.json`. A
snippet never causes a connection that dictation alone would not.

## Connections

- An expansion can be an **AI instruction** rather than a literal block (the "organize my
  thoughts" use case). That is really a saved prompt — see [quick-transforms](quick-transforms.md),
  which is the better home for prompt-shaped snippets so the two do not blur.
- Triggers benefit from the same whole-word matcher the dictionary already uses; share the
  helper rather than writing a second one.

## Open questions

- Case handling: should "MY EMAIL" expand differently from "my email"? Probably no — expansions
  are verbatim.
- A `{caret}` placeholder so the cursor lands inside the expansion (e.g. a code block) — nice,
  but needs caret control we do not have via paste; defer.
- Import/export: snippets are plain JSON in settings already; a "Show in Finder" affordance (as
  used for profiles) is likely enough, no dedicated importer.
- Sharing across a team is **out** — that needs a service, and OpenFlow has none by design.
