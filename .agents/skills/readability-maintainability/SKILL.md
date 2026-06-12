---
name: readability-maintainability
description: Improve naming, comments, file organization, and module size anywhere in Velata — or review a diff for readability. Use this whenever you write more than a screen of new code, rename things, add or delete comments, or feel tempted to split or merge files, in either Rust or TypeScript.
---

# Readability and maintainability

## Purpose

Optimize code for the next reader — a contributor or an agent who has not
seen this file before. The repo's bias is boring and explicit: small named
functions over clever blocks, domain words over generic ones, comments that
carry constraints rather than narration.

## Files to inspect first

- `docs/engineering/monorepo-conventions.md` — naming, comments, and
  when-to-split rules (authoritative).
- The file you're editing, fully — match its idiom, comment density, and
  test style before changing it.

## Procedure

1. **Names.** Replace vague names (`data`, `item`, `temp`, `manager`,
   `helper`, `util`) with domain words. Rust getters drop `get_`;
   conversions signal cost (`as_`/`to_`/`into_`); hooks start with `use`
   only if they call hooks. Don't rename wire-visible IPC names or public
   API purely for style — that's churn.
2. **Comments.** Keep only the ones that say _why_: invariants, OS quirks,
   tradeoffs, failure modes, ordering requirements. Delete restatements of
   the next line. If your change makes a nearby comment stale, fix it in the
   same diff. Keep `(07 §4)`-style spec references — they're the paper trail
   into `docs/design/`.
3. **Doc comments.** Public Rust items and shared TS types get a one-line
   summary, third person, declarative ("Returns …"). Don't restate the
   signature; name the thing the signature can't say (side effects,
   blocking, idempotence).
4. **Long procedures.** Extract a _named_ step when the name adds meaning at
   the call site. Leave honest sequential code alone — five trivial
   single-caller functions are worse than one readable block.
5. **File size.** Split only on mixed responsibilities, never on line count.
   Check the documented seams first; if you find a new genuine seam, record
   it in `monorepo-conventions.md` rather than splitting opportunistically.
6. Run the relevant gate (below) — `noUnusedLocals` and clippy catch the
   debris a cleanup leaves behind.

## Rules

- Behavior-preserving. A readability PR that changes behavior buries the
  change where nobody will look for it.
- Dead code is deleted, not commented out, and only after grepping call
  sites (including tests and `generate_handler!`).
- Error message text is user-facing copy in this app (HUD notices) — editing
  it is a UX change, not a comment tweak.
- One concern per PR: don't mix a rename sweep with logic changes.

## Commands

```sh
pnpm check        # TS side, includes lint with unused checks
pnpm check:rust   # Rust side, clippy -D warnings catches dead code
```

## Checklist

- [ ] No vague names introduced; no cosmetic renames of public/wire names
- [ ] Every kept comment states a constraint or reason; stale ones fixed
- [ ] New public items documented (one line, third person)
- [ ] Splits/extractions justified by responsibility, not size
- [ ] Gates green; behavior demonstrably unchanged

## Common mistakes

- "Tidying" a why-comment into oblivion and leaving the constraint
  undocumented (the HUD never-hide note has survived three such attempts).
- Renaming an IPC command or serde field for naming purity — that's a
  breaking wire change.
- Extracting a helper used once, three lines long, named `process_data`.
- Adding section-banner comments (`// ---- handlers ----`) instead of
  structure.

## Expected output

A diff a stranger can review quickly: better names, fewer but sharper
comments, documented public items, no behavior change, all gates green.
