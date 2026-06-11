# 05 — Discoverability and feature education

Status: design proposal. Written 2026-06-11 against `main` (cd63494) + the in-flight Refine
work (`docs/REFINE.md`). Baseline facts and vocabulary: `00-current-state.md`. Structure and
flows (F1–F12): `03-information-architecture.md`. This document owns everything that teaches a
feature _after_ onboarding.

**Handoff note.** `02-ux-audit.md` and `04-onboarding.md` did not exist when this was written.
This doc assumes the split agreed in the brief: **onboarding teaches the irreducible basics**
— hold-to-dictate, the < 350 ms tap-latch, the Rewrite hotkey, and where modes live. _Everything
else is this document's job._ If 02/04 land later with conflicting handoff lines or audit IDs
(UX-xx) this doc should cite, treat those as the authority and reconcile. Audit IDs are
referenced below as `(UX-??)` placeholders where an audit finding is the obvious motivation.

Design goal: every capability answers three questions at the right moment — **what it does,
why you'd use it, when to use it** — using only progressive disclosure, empty states, hint
lines, inline explanations, example prompts, one-time contextual tips, and the HUD success
flash. No videos, no web-docs dependency, **no telemetry** — every tip trigger is computed
from local settings state (00 §8.1).

---

## 1. The feature-education gap (the spine)

Every shippable capability × what teaches it today × the proposed mechanism and moment. "Mode"
column values come from 03 §2's sitemap pages. Deep-link targets are 03 settings pages.

| #   | Capability                                                  | Taught today by                                                      | Proposed mechanism                                                                                       | Moment / trigger                                  |
| --- | ----------------------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| 1   | Dictation (hold)                                            | Onboarding step 1+5; General hint                                    | Onboarding (keep). Cheat-sheet row.                                                                      | First run; reference any time                     |
| 2   | Tap-latch hands-free                                        | General hint only ("a quick tap latches…") (UX-??)                   | Onboarding line + cheat-sheet + **tip T-LATCH**                                                          | After dictations that were all holds, never a tap |
| 3   | Dictation style (hold ↔ toggle)                             | Row label + hint "How the dictation hotkey behaves."                 | Sharper hint (§4)                                                                                        | In place, on the Dictation page                   |
| 4   | Esc / cancel                                                | **Nothing** — `Esc` is not even bound (00 §5)                        | Bind `Esc`; cheat-sheet row; HUD shows "Esc to cancel" while recording (§6, F-future)                    | While recording (HUD hint)                        |
| 5   | Rewrite selection                                           | General hint "Select text anywhere, hold, and speak an instruction." | Onboarding mention + cheat-sheet + **example prompts** (§5)                                              | First run; placeholder in editor                  |
| 6   | Polish selection (in flight)                                | REFINE hotkey row hint "Fix grammar and clarity. No voice."          | Cheat-sheet + **tip T-POLISH**                                                                           | After ≥1 Rewrite used, Polish never               |
| 7   | Empty-instruction → default polish                          | REFINE.md prose only; **no UI** (UX-??)                              | One line under the Rewrite editor placeholder (§5)                                                       | In the Rewrite "what to say" disclosure           |
| 8   | Modes + switching surfaces                                  | Tray list; Modes page hint                                           | **Tip T-MODES**; Modes page intro line                                                                   | After 3rd successful dictation, no custom mode    |
| 9   | Mode templates                                              | **Nothing** (templates are new in v2, 03 §2)                         | Modes empty/active state "Browse templates…" entry (§3)                                                  | On the Modes page, always visible                 |
| 10  | Per-mode overrides (AI profile / model / language / hotkey) | **Nothing** (new, 03 D1)                                             | "Advanced" disclosure in mode editor + hint copy (§4)                                                    | Inside the mode editor, collapsed                 |
| 11  | Dictionary                                                  | Tab explanation line                                                 | Keep line; **ghost example row** in empty state (§3); **"Add correction" quick win** on Last result (F4) | Empty Dictionary; after a misheard word           |
| 12  | AI profiles                                                 | REFINE.md empty-state line + editor                                  | Keep REFINE.md empty state **verbatim** (§3); **tip T-AI**                                               | After N dictations, no profile, refine on         |
| 13  | Refine-with-AI toggle                                       | REFINE row + tray item                                               | Hint copy (§4); tray label                                                                               | In place; deep-link target of T-AI                |
| 14  | Copy Last Result                                            | Tray item; Last result card "Copy"                                   | Cheat-sheet row; recovery is taught by the HUD clipboard notice                                          | On paste failure (HUD) + always in tray           |
| 15  | Insert fallback (clipboard)                                 | Onboarding step 3; GeneralOutput hint                                | HUD outcome notice "Copied to clipboard — press ⌘V" (REFINE §Insertion)                                  | When paste degrades, at the moment it happens     |
| 16  | Spoken language                                             | Row hint "English-only models ignore this."                          | Sharper hint + per-mode override pointer (§4)                                                            | In place, on the Dictation page                   |
| 17  | Speech model choice                                         | Model rows: size + description                                       | Tradeoff-in-seconds hint (09); **tip T-ACCURACY**                                                        | After a "Didn't catch anything" notice streak     |
| 18  | Launch at login                                             | Row hint                                                             | Keep                                                                                                     | In place (General)                                |

Three capabilities have **zero** affordance today and are the highest-leverage gaps: **Esc /
cancel (#4)**, **mode templates (#9)**, and **per-mode overrides (#10)**. The empty-instruction
default-polish behavior (#7) ships invisibly and surprises users; it gets one inline line.

---

## 2. The tip system (the only new machinery)

One-time contextual tips. The single piece of net-new infrastructure. Everything else reuses
existing card/hint/HUD surfaces.

### 2.1 Principles

- A tip is **shown at most once, ever** (then its id is in `tipsSeen`). A missed tip is never
  lost information — the cheat-sheet (§6) and empty states (§3) restate everything a tip says.
- Tips are **triggered by local state only** (counts and config facts), never by usage
  analytics. No network, no event log.
- **Two surfaces, both already constraint-safe:**
  - **Settings card tip** — a dismissible `<TipCard>` pinned to the top of the relevant
    settings page. Interactive (has an action button that deep-links).
  - **HUD post-success flash line** — one extra line appended to the success flash after an
    insert completes. The HUD is click-through and feedback-only (00 §8.2), so this is
    **text-only, never a button**. It points; it cannot act.
- **Frequency cap:** at most **one tip per calendar day**, and a **hard cap of one tip per
  app session** regardless of date. A global toggle kills all of them.

### 2.2 Storage (additive schema change)

Add to `Settings` (00 §8.3 mirror discipline applies — change `settings.rs` and
`packages/core/src/types.ts` in the same PR; all fields additive, defaulted, so migration is
a no-op beyond bumping nothing — these read as their defaults on old files):

| Field            | Type       | Default | Meaning                                                                |
| ---------------- | ---------- | ------- | ---------------------------------------------------------------------- |
| `tipsEnabled`    | `boolean`  | `true`  | Global "Show tips" master switch (General page)                        |
| `tipsSeen`       | `string[]` | `[]`    | Tip ids already shown; never re-shown                                  |
| `dictationCount` | `number`   | `0`     | Successful dictations ever (incremented on `dictation` insert success) |
| `lastTipShownAt` | `string`   | `""`    | ISO-8601 date (`YYYY-MM-DD`) of the last tip shown; enforces ≤ 1/day   |

`dictationCount` is the only counter; it increments in `pipeline.rs` when a `dictation` job
reaches a successful insert (not Polish/Rewrite, not failures). It is a count, never a log —
no timestamps per dictation, nothing reconstructable. Resetting "Show tips" (below) clears
`tipsSeen` and `lastTipShownAt`; it does **not** touch `dictationCount` (that is product
telemetry-free state, not a tip artifact). `dictationCount` saturates conceptually — once past
the largest threshold (6) it only matters that it is ">", so no overflow concern.

### 2.3 Evaluation point

Tips are evaluated at exactly **two cheap moments**, never on a timer and never during the
dictation hot path:

1. **Settings webview gains focus** (`App.tsx` mount + window-focus event) — evaluate the
   _settings-card_ tips for the page being shown. This is when the user is already reading UI.
2. **Pipeline success event** for a `dictation` job — the Rust side has just bumped
   `dictationCount`; it includes a resolved `hudTip: string | null` field on the success
   payload, computed once, so the HUD shows the flash line without the webview re-deriving it.
   (HUD tips are chosen Rust-side because the HUD webview has no settings access by design.)

No tip is ever evaluated while `status` is `recording`/`transcribing`/`refining`/`inserting`
(anti-annoyance, §7). The HUD tip rides the success flash that already exists in 04/09.

### 2.4 Tip catalog

Predicates are boolean expressions over settings + `dictationCount`. `once` = id not in
`tipsSeen`. All tips additionally require `tipsEnabled && lastTipShownAt != today && no tip
already shown this session`. Copy is ≤ 2 sentences. Action buttons exist only on settings-card
tips; HUD tips are text-only.

| id               | Trigger predicate                                                                                                                                                 | Surface                                           | Copy                                                                                                                  | Action → deep link                     | Dismiss                     |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | --------------------------- |
| `tip.latch`      | `dictationCount == 3 && behavior=='hold'` and (no tap-latch has been observed — approximated as: still on `hold` after 3 dictations)                              | HUD flash line                                    | "Tip: tap and release the dictation key — don't hold — to keep recording hands-free. Tap it again to stop."           | — (text only)                          | Auto after flash; id stored |
| `tip.modes`      | `dictationCount >= 3 && modes.every(m => m.builtIn)`                                                                                                              | Settings card on **Dictation**                    | "Modes change how your words come out — Email, Notes, code, or your own. Pick one in the menu bar, or make your own." | "Open Modes" → `S:Modes`               | × button; id stored         |
| `tip.ai`         | `dictationCount >= 4 && activeLlmProfileId=='' && refineAfterDictation==true`                                                                                     | Settings card on **Dictation**                    | "Want sharper cleanup? Add a local Ollama model or your own API key and OpenFlow will polish transcripts with AI."    | "Set up AI" → `S:Models` (AI profiles) | × button; id stored         |
| `tip.polish`     | `polishHotkey set && dictationCount >= 2 && (has used Rewrite — see note) && activeLlmProfileId != ''`                                                            | HUD flash line (after a Rewrite)                  | "You can also tap ⌥⇧P to fix grammar in any selected text — no talking needed."                                       | — (text only)                          | Auto after flash; id stored |
| `tip.dictionary` | `dictionary.length == 0 && dictationCount >= 5`                                                                                                                   | Settings card on **Output** (next to Last result) | "If a name or term comes out wrong a lot, teach OpenFlow the fix once in the Dictionary."                             | "Open Dictionary" → `S:Dictionary`     | × button; id stored         |
| `tip.accuracy`   | `sttModelId is an `.en`/base tier && two consecutive "Didn't catch anything"/low-confidence notices` (counted in a transient in-memory streak, **not** persisted) | Settings card on **Models**                       | "Struggling with accuracy? A larger speech model hears more — at the cost of a second or two per dictation."          | "Choose a model" → `S:Models`          | × button; id stored         |

**Catalog size: 6 tips.** Three HUD-eligible (`latch`, `polish`, and none others are text-only
by nature), three settings-card. `tip.accuracy`'s streak is the one signal not derivable from
persisted settings; it lives as a runtime counter in `pipeline.rs` (reset on any success) and
is therefore _not_ a stored field — it never survives a restart, which is acceptable because
the tip is a nicety, not load-bearing.

**Note on "has used Rewrite/tap" signals.** OpenFlow persists no usage history (00 §6, privacy
feature). `tip.latch` and `tip.polish` therefore approximate from settings facts rather than
true usage: `tip.latch` fires off the _configured behavior still being hold_ after 3
dictations; `tip.polish` is gated to fire only as the flash line **immediately following a
Rewrite job's success** (so the "you used Rewrite" fact is the live event, not stored). Neither
introduces a new persisted counter. This is deliberate: we accept a slightly less precise
trigger to avoid logging what the user does.

### 2.5 Global control and reset

- **General page** gets one row: **Show tips** `[toggle]` bound to `tipsEnabled`, hint:
  "One-time hints about features you haven't tried. Never repeats, never interrupts dictation."
- Directly beneath, a quiet link **"Reset tips"** → sets `tipsSeen: []`, `lastTipShownAt: ""`.
  Confirmation copy: "Show all feature tips again?" This is the reset story — pair it visually
  with the existing "Welcome tour → Show again" (03 §2 General) since both re-surface teaching.

---

## 3. Empty states

Exact copy + the single primary action for each. Empty states must contain everything the
matching tip says (§7), so they are the durable home of the lesson.

### 3.1 Dictionary (teach from→to with a ghost row)

The current empty state is one line, "No entries yet." (DictionaryTab.tsx). Replace with a
**ghost example row** — a non-interactive, dimmed sample that shows the from→to shape — plus
the existing explanatory line (which is already good, keep it):

> Heading: **Personal dictionary**
> Hint (keep): "Fix words the transcriber keeps getting wrong — names, products, jargon."
> Ghost row (dimmed, `aria-hidden`, not removable): `open flow  →  OpenFlow`
> Below the ghost: "Add your first correction above."

Primary action: the existing `from → to` add form (already present). The ghost row replaces
"No entries yet." and is hidden the moment `dictionary.length > 0`.

### 3.2 Modes (templates entry point)

The Modes list is never truly empty (built-ins always exist), so the "empty state" here is
**the absence of a custom mode**. Add an intro line above the list and a templates button
beside "New mode":

> Intro line: "Modes shape how your dictation is written — pick one in the menu bar, or
> create your own from a template."
> Buttons (row): **Browse templates…** (primary; opens the gallery sheet, 03 §2) · New mode

When `modes.every(m => m.builtIn)` is true, "Browse templates…" is styled as the primary
call-to-action; once a custom mode exists it becomes a normal button. Template gallery item
copy is owned by `06-custom-instructions.md`; this doc only specifies the entry point.

### 3.3 AI profiles (keep REFINE.md's empty state verbatim)

REFINE.md already designs this and it is good — **keep it exactly**, only re-homed to
Models → AI profiles (03 §3). Quoted from REFINE.md:

> "Dictation uses fast rules-based cleanup. Add a profile to enable AI polish and the
> selection shortcuts." — plus the New profile button.

No change. This sentence already answers what/why/when in two clauses. Do not rewrite it.

### 3.4 Last result (before first dictation)

The Last result card only appears after a dictation today (it is gated on `lastResult`). Keep
that gate — an empty Last result card is noise. But the **Output page** should not look broken
when there is nothing yet, so show a one-line placeholder in the card's slot:

> "Your most recent dictation will appear here — handy if a paste lands in the wrong app."

Replaced by the real card (final text · raw transcript · Copy · **Add correction**, F4) the
moment a result exists. The **"Add correction"** button is the F4 quick win: it opens the
Dictionary add-form pre-filled with `from` = the raw transcript token(s) and `to` = the final
text, turning a misheard word into a 1-click fix.

### 3.5 History (future, one line)

History is roadmap (opt-in, 03 §4). When it lands, its empty state is one line — specified now
so the slot reads consistently:

> "History is off. Turn it on in General to keep a local, searchable log of past dictations —
> stored only on this Mac."

---

## 4. Inline explanation pass (the hint-line layer)

**Rule:** a row gets a hint **only when the label alone is ambiguous about what the control
does or what the consequence is.** A self-explanatory label (e.g. "Launch at login") needs no
hint. Hints are ≤ one short sentence (03 §4 page anatomy). Below: every 03-sitemap row that
needs a hint, the exact copy, and a flag for rows whose _current_ hint is wrong/unhelpful vs
merely missing.

| Page → row                               | Status                                                                                                                            | Exact hint copy                                                                                        |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Dictation → Dictation (hotkey)           | keep                                                                                                                              | "Hold and speak; release to insert. A quick tap latches hands-free mode."                              |
| Dictation → Dictation style              | **sharpen** (current "How the dictation hotkey behaves." is vague — UX-??)                                                        | "Hold to talk = record while held. Press to start/stop = tap once to begin, again to end."             |
| Dictation → Polish selection             | keep (REFINE)                                                                                                                     | "Fix grammar and clarity in selected text. No voice."                                                  |
| Dictation → Rewrite selection            | keep                                                                                                                              | "Select text anywhere, hold, and speak an instruction."                                                |
| Dictation → Spoken language              | **sharpen** (current "English-only models ignore this." states a limit but not the override)                                      | "The language you speak. English-only models ignore this. A mode can override it."                     |
| Dictation → Refine with AI               | keep (03)                                                                                                                         | "Polish transcripts with your active mode and AI profile. Off = fast rules-based cleanup, no network." |
| Modes → list intro                       | add                                                                                                                               | (see §3.2 intro line)                                                                                  |
| Modes → editor → Uses AI                 | **fix** (current "Send the transcript to your AI provider with this prompt." assumes a profile exists and predates "AI profiles") | "Send this mode's transcripts through your active AI profile. Off = rules-based cleanup only."         |
| Modes → editor → Advanced (disclosure)   | add                                                                                                                               | "Override the global AI profile, speech model, language, or give this mode its own hotkey."            |
| Modes → editor → Advanced → AI profile   | add                                                                                                                               | "Which AI connection this mode uses. Inherit = your globally active profile."                          |
| Modes → editor → Advanced → Speech model | add                                                                                                                               | "Which speech model this mode uses. Inherit = your global model."                                      |
| Modes → editor → Advanced → Mode hotkey  | add                                                                                                                               | "Press this key to dictate straight into this mode, skipping the menu."                                |
| Models → AI profiles → row badge         | keep (REFINE)                                                                                                                     | local/cloud badge derived from URL; no text hint needed                                                |
| Output → Insert method                   | **sharpen** (current "Paste needs the Accessibility permission." is a footnote, not a description)                                | "How text reaches your app. Paste needs Accessibility; clipboard-only always works."                   |
| Output → Restore clipboard               | keep                                                                                                                              | "Put your previous clipboard back after pasting."                                                      |
| Dictionary → card                        | keep                                                                                                                              | "Fix words the transcriber keeps getting wrong — names, products, jargon."                             |
| General → Launch at login                | keep                                                                                                                              | "Start OpenFlow in the menu bar when you sign in."                                                     |
| General → Show tips                      | add (§2.5)                                                                                                                        | "One-time hints about features you haven't tried. Never repeats, never interrupts dictation."          |

Rows deliberately **without** a hint: Models → speech model rows (the size + description line
is the hint), About rows (self-evident), Output → Last result (it is a result, not a control).

---

## 5. Example prompts ("what can I say?")

Two editors need example content: the **mode prompt** editor and the **Rewrite instruction**.
The mode prompt is a system prompt (developer-facing); the Rewrite instruction is a spoken
sentence (end-user-facing). They get different treatments.

### 5.1 Rewrite instruction — a "What can I say?" disclosure

The Rewrite hotkey has no editor; the user _speaks_ the instruction. So the examples live as a
collapsible **"What can I say?"** disclosure beside the Rewrite row on the Dictation page, plus
they seed the empty Last-result placeholder. Curated spoken-instruction list (8–10):

1. "Make it shorter."
2. "Make this more polite."
3. "Fix the grammar."
4. "Turn this into bullet points."
5. "Translate to German."
6. "Make it sound more formal."
7. "Rewrite this as an email."
8. "Summarize in one sentence."
9. "Fix the spelling and punctuation only."
10. "Make it friendlier."

Below the list, the **empty-instruction behavior** (gap #7) gets its one inline line:

> "Say nothing and release — OpenFlow just cleans up grammar and clarity."

### 5.2 Mode prompt editor — rotating placeholder

When a custom mode's prompt is empty (or on a fresh "New mode"), the textarea shows a rotating
`placeholder` (not pre-filled text — placeholders don't save). Rotate among:

- "Clean up dictated speech. Fix punctuation, remove fillers. Output only the text."
- "Format as a Slack message: casual, concise, an emoji if it fits."
- "Rewrite as a professional email with a greeting and sign-off."
- "Turn rambling speech into tight bullet points."

A one-line helper sits under the textarea: "Describe how this mode should rewrite your words.
The transcript is added automatically — just give the instruction." Full template prompts are
06's domain; these placeholders are the lightweight nudge for the from-scratch path.

---

## 6. Keyboard cheat-sheet

**Decision: a single static "Keyboard shortcuts" card on the General page**, not About, not a
tray submenu.

Justification:

- **Not About** — About is identity/metadata (version, paths, licenses). Shortcuts are
  operational; users won't look under "About" for "how do I cancel?".
- **Not the tray** — the tray is the quick-switch surface (03 §4); a static reference submenu
  would consume click-through real estate and can't show rebindings cleanly. The tray already
  links to Settings.
- **General** — it already hosts "Welcome tour → Show again" and "Show tips" (§2.5), so all the
  "learn / re-learn the app" affordances cluster on one page. The card reads the _live_ bound
  accelerators from settings, so rebindings stay accurate (no hardcoded keys).

Card content (values are live from settings; `⌥⇧P`/etc. shown via `formatAcceleratorMac`):

> **Keyboard shortcuts**
> Dictate — hold {dictationHotkey} (or tap to latch hands-free; tap again to stop)
> Cancel — Esc while recording
> Rewrite selection — hold {refineHotkey}, then speak the change
> Polish selection — tap {polishHotkey}
> Copy last result — from the menu bar, "Copy Last Result"

This card is the **durable backstop** for every tip: anything a tip teaches (latch, Polish,
recovery) is permanently readable here, satisfying the §7 "a missed tip is never lost"
guarantee. The "Esc while recording" row depends on binding Esc (gap #4); until then it reads
"Cancel — start a new dictation or quit from the menu bar" to stay truthful.

---

## 7. Anti-annoyance rules

Hard rules. A violation is a bug.

1. **No modal tours after onboarding.** No full-screen takeovers, no carousels. Tips are inline
   cards or one HUD line.
2. **No badges, red dots, or "NEW" pills** on sidebar items or rows. Discoverability comes from
   empty states and tips, not nagging ornaments.
3. **No tip repeats.** Once an id is in `tipsSeen` it never shows again (until an explicit
   "Reset tips").
4. **No tip during dictation.** Tips never evaluate or render while the pipeline is
   recording/transcribing/refining/inserting. The HUD flash tip appears only _after_ a
   successful insert, never mid-flow.
5. **At most one tip per day and one per session**, enforced by `lastTipShownAt` and a
   session flag (§2.1).
6. **Instant global opt-out.** "Show tips" off (§2.5) silences every tip immediately, this
   session included.
7. **Nothing is tip-only.** The cheat-sheet (§6) and empty states (§3) restate every lesson, so
   a dismissed or never-triggered tip loses the user no information.
8. **HUD tips are text, never controls** (00 §8.2). They point at a feature; they cannot be
   clicked. Acting on them is done at leisure in Settings.

---

## 8. Implementation notes

### 8.1 Components touched

| File                                                | Change                                                                                                                                              |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `packages/core/src/types.ts`                        | Add `tipsEnabled`, `tipsSeen`, `dictationCount`, `lastTipShownAt` to `Settings`; add `hudTip: string \| null` to the dictation success payload type |
| `apps/desktop/src-tauri/src/settings.rs`            | Mirror the four fields (camelCase, defaulted); additive — old files read defaults                                                                   |
| `apps/desktop/src-tauri/src/pipeline.rs`            | Increment `dictationCount` on dictation insert success; compute `hudTip` for HUD-eligible tips; runtime accuracy-streak counter                     |
| `src/app/components/TipCard.tsx` (new)              | Dismissible card: copy + optional action button (deep-link) + ×                                                                                     |
| `src/app/tips.ts` (new)                             | The catalog (§2.4) as pure predicates over `(settings, dictationCount)`; one `nextSettingsTip(page, settings)` selector applying the frequency cap  |
| `src/app/App.tsx`                                   | Evaluate settings-card tips on focus; render `<TipCard>` at the top of the active page; deep-link target plumbing (stored target from 03 §4)        |
| `src/app/Hud.tsx` / `hudState.ts`                   | Append the `hudTip` line to the success flash; text-only                                                                                            |
| `GeneralTab.tsx` (→ Dictation/General split per 03) | "Keyboard shortcuts" card (§6); "Show tips" toggle + "Reset tips" link (§2.5)                                                                       |
| `DictionaryTab.tsx`                                 | Ghost example row empty state (§3.1)                                                                                                                |
| `ModesTab.tsx`                                      | Intro line + "Browse templates…" entry (§3.2); rotating prompt placeholder (§5.2)                                                                   |
| `tabs/OutputTab.tsx` (new per 03)                   | Last-result placeholder line + "Add correction" button (F4, §3.4)                                                                                   |
| Rewrite area on the Dictation page                  | "What can I say?" disclosure (§5.1)                                                                                                                 |

No new persistence store, no new window type. Deep links reuse the stored-target mechanism 03
§4 already defines for HUD error notices.

### 8.2 Tip-evaluation point (restated)

Settings-card tips: **on settings webview focus**, for the visible page only. HUD tips:
**computed Rust-side on the dictation success event**, shipped as `hudTip`. Never on a timer,
never during the hot path (§7 rule 4).

### 8.3 Build order — ship these 5 first for ~80% of the value

1. **Cheat-sheet card (§6)** — zero new state, makes Esc/Polish/latch/recovery permanently
   discoverable. Highest value-to-effort.
2. **Dictionary ghost row + "Add correction" on Last result (§3.1, §3.4 / F4)** — turns the
   most common frustration (misheard word) into a one-click fix; pure UI.
3. **Modes empty state → "Browse templates…" (§3.2)** — the single biggest unexposed feature
   (#9) gets a permanent door; pairs with 06.
4. **Hint-line pass (§4)** — fixes the wrong/vague hints (Dictation style, Spoken language,
   Uses AI, Insert method) and adds the Advanced-override hints; copy-only, no logic.
5. **Tip system core (§2) with `tip.modes` + `tip.ai` only** — the two settings-card tips that
   most move new users toward the product's depth; the HUD tips and `tip.accuracy` follow once
   04/09's success flash and label work land.

Everything else (example-prompt disclosure, rotating placeholders, remaining tips, Esc binding)
layers on without reopening the schema — the four settings fields in §2.2 are the only data
contract this whole document adds.
