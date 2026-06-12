# 07 — Persona bundles: Mode v2, overrides, switching, resolution

Status: design proposal. Written 2026-06-11 against `main` (cd63494) + the in-flight Refine
work (`docs/REFINE.md`, `apps/desktop/src-tauri/src/profiles.rs`). Baseline facts and vocabulary:
`00-current-state.md`. Foundation decision: `03-information-architecture.md` D1 (modes are the
one switchable bundle). Prompts, templates, and sharing belong to `06-custom-instructions.md`
("see 06"); this document owns bundling, overrides, switching, and resolution. This is the full
design of D1.

The worktree this lands in is still at settings schema v1. The Refine work (profiles, the
`activeLlmProfileId` pointer, `refineAfterDictation`, `polishHotkey`, the 3-hotkey rollback)
takes settings to v2 and ships first. **Everything below builds on v2 and bumps it to v3.**

---

## 1. Why one concept, not two

The brief asks for one-click-switchable **Profiles** — Developer, Student, Writer, Researcher,
Executive, Custom — each bundling an LLM, an STT model, instructions, an output format, and a
shortcut. Velata already ships exactly one switchable bundle: the **Mode**. D1 (03) decided
to grow the Mode into that bundle rather than add a second concept. This section defends that.

A separate `Profile` entity beside `Mode` would cost, concretely:

- **A 2-axis active-state matrix.** Today one fact answers "what will happen when I dictate?":
  the active mode. A second axis forces the user (and every support thread) to reason about
  _mode × profile_ — "Notes mode under the Developer profile" — and to find the one combination
  that does what they want. 00 §10 makes the active state visible at a glance; two axes break that.
- **Double debugging paths.** "Why did it write that?" already resolves through mode + active AI
  profile + dictionary. A persona layer on top adds a second resolver that can disagree with the
  first (profile says cloud, mode says local). Two resolvers means two places every bug hides.
- **Two tray sections.** The tray is the quick-switch surface (00 §10, 03 §4). A radio list of
  modes _and_ a radio list of profiles doubles its height and forces the user to keep two
  selections in sync from a menu that is meant to be a one-click switch.
- **Settings duplication.** A profile that bundles "instructions + output format" overlaps the
  Mode's prompt and the Output page. Two homes for the same setting means edits in one silently
  lose to the other.

**Superwhisper precedent.** The best-regarded customization model in this category has exactly
one such concept, also called a _mode_, bundling STT model + AI model + prompt + activation key +
output behavior (01). It did not grow a "profile" on top of "mode"; the mode _is_ the bundle. We
follow that, keeping our vocabulary (00 §7): the bundle is a **Mode**, "AI profile" stays the LLM
connection, "Polish/Rewrite" stay actions.

### Brief personas → shipped mode templates

The brief's six personas ship through 06's gallery, which mints **task-named templates carrying
a persona tag** (`commit · developer`, `study-notes · student`, …) — concrete jobs beat abstract
persona labels when a user scans a gallery. A persona is therefore one or two gallery picks plus
the override suggestions in §2, not new machinery. The prompts and the gallery live in 06.

| Brief "Profile" | Ships as (06 catalog ids + built-ins)           | Built on              |
| --------------- | ----------------------------------------------- | --------------------- |
| Developer       | built-in **Literal** + `commit`                 | exact words + commits |
| Student         | `study-notes` (+ `academic`)                    | Notes-style bullets   |
| Writer          | `email` + `social-post` (+ `academic`)          | clean prose           |
| Researcher      | `meeting-notes` + a multilingual STT suggestion | Notes + best capture  |
| Executive       | `email` + `meeting-notes`                       | cloud-quality prose   |
| Custom          | **New mode** / Duplicate                        | the from-scratch path |

There is no "persona" object anywhere in the schema. "Persona" is the gallery tag that groups
task templates for a kind of user, nothing more.

---

## 2. The persona bundles

Each persona is a gallery pick: a prompt (owned by 06) plus the four Mode v2 overrides
(`aiProfileId`, `sttModelId`, `language`, `hotkey`). The "Primary template" column references
06's shipped catalog ids (built-in modes where the persona needs no template); §1 lists the
companion picks.

Override columns use the **value the template ships with**. `null` means "inherit the global
setting" and is the right answer whenever the persona has no opinion — inheriting keeps the mode
working when the user changes their global model or language later. We only pin an override when
the persona's value statement depends on it.

`AI profile kind` is a _suggestion shown in the gallery_, not a stored value — a template cannot
ship a machine-local profile id (those don't travel, §8). It tells the user which kind of AI
profile to point the mode at, privacy-tiered: **local** (Ollama / LM Studio, nothing leaves the
machine) vs **cloud** (BYO-key, text-only).

| Persona        | Primary template                | usesLlm   | STT model override                                                     | language | AI profile kind (suggested) | hotkey | Value statement (user sees)                                                         |
| -------------- | ------------------------------- | --------- | ---------------------------------------------------------------------- | -------- | --------------------------- | ------ | ----------------------------------------------------------------------------------- |
| **Developer**  | built-in `Literal` (+ `commit`) | **false** | `null`                                                                 | `null`   | none (no AI)                | none   | "Exact words, no rewriting — code, commands, and identifiers stay literal."         |
| **Student**    | `study-notes`                   | true      | `null`                                                                 | `null`   | local                       | none   | "Lecture notes as tidy bullets, on-device — nothing leaves your laptop."            |
| **Writer**     | `email` (+ `social-post`)       | true      | `null`                                                                 | `null`   | local                       | none   | "Spoken drafts become clean prose; tone preserved."                                 |
| **Researcher** | `meeting-notes`                 | true      | suggest multilingual (`large-v3-turbo-q5_0`) if installed, else `null` | `null`   | cloud                       | none   | "Detailed notes from interviews and talks, best-quality transcription and cleanup." |
| **Executive**  | `email` + `meeting-notes`       | true      | `null`                                                                 | `null`   | **cloud**                   | none   | "Polished email and meeting notes, fast — cloud-quality phrasing."                  |
| **Custom**     | — (`New mode`)                  | true      | `null`                                                                 | `null`   | inherit                     | none   | "Start from scratch; add overrides only when you need them."                        |

Why these choices, persona by persona:

- **Developer ≈ Literal / Coding, no-AI default** (`usesLlm: false`). A developer dictating a
  variable name or a shell command wants the exact words, not an LLM "improving" `git rebase -i`
  into prose. No-AI is also the only output guaranteed to make zero network calls (REFINE.md), which
  a privacy-minded developer expects. The AI-profile row is disabled here (§8) — the override would
  do nothing.
- **Student ≈ Notes, local.** Bullets fit lecture capture. Local AI beats cloud because students
  dictate volume on metered or campus networks, and the privacy default should win when "good
  enough" suffices — Notes cleanup is well within a local 3B model's reach.
- **Writer ≈ Email-style prose, local.** Same local-first logic; prose cleanup does not need a
  frontier model. The prompt (06) shapes prose, distinct from Email's greeting/sign-off handling.
- **Researcher ≈ Notes + best transcription.** The one persona that _suggests an STT override_:
  interviews are long, accented, and multilingual, so a multilingual model earns its size here where
  it is overkill for a developer dictating English commands. The override stays a _gallery
  suggestion_, not a pinned value — pinning `large-v3-turbo-q5_0` would dangle on a machine that
  only downloaded `base.en` (§3). Cloud AI is suggested for synthesis quality; BYO-key.
- **Executive ≈ Email + Meeting Notes, cloud-quality.** The one persona where cloud is the
  _default_ suggestion: executives optimize for output quality and speed over local-only purity, and
  are the segment most likely to already hold an API key. "Cloud-quality" is said out loud so the
  privacy trade is explicit.
- **Custom** is the from-scratch path: "New mode" (06) or duplicate any template. All overrides
  start `null`, so a fresh custom mode behaves exactly like today's (03 D1).

None of these pin a `hotkey`: a shipped hotkey would collide across users who install several
personas, and the cap (§4) is small. The user assigns mode hotkeys themselves to the two or three
modes they switch between most.

---

## 3. Override resolution

Resolution happens **once per job, at job start**, inside `pipeline.rs`. It does not re-read
settings mid-pipeline. This preserves the generation-counter contract (00 §8.4): the job captures
its resolved inputs at `start()`, and a later settings change only affects the _next_ job, never
the one in flight. Concretely, the resolver runs where `process()` currently reads
`settings.stt_model_id` / `settings.language`, and where `finish_dictation` currently reads the
active mode and provider.

### Precedence (per field)

For each of `{AI profile, STT model, language}`:

```
mode override (if set AND valid)  →  global setting  →  built-in default
```

- **AI profile.** `mode.aiProfileId` if non-null and the profile file exists →
  `settings.activeLlmProfileId` (the globally active profile, possibly "No AI") → No AI. ANDed,
  as today, with `mode.usesLlm` and the `refineAfterDictation` master switch (REFINE.md): if the
  mode is no-AI or the master switch is off, no profile is resolved at all.
- **STT model.** `mode.sttModelId` if non-null and **installed** → `settings.sttModelId` if
  installed → the start-of-job model check fails exactly as it does today
  ("speech model not downloaded yet — open Settings to install it"). The override never bypasses
  the installed-model gate.
- **Language.** `mode.language` if non-null → `settings.language` → `"auto"`. No installation
  concept; any ISO 639-1 string or `auto` is valid.

"Valid" means: the override is non-null **and** the referenced thing still resolves — the profile
file exists (ProfileManager already returns `None` for a dangling id, `profiles.rs::active`), the
STT model is installed (`ModelManager::is_installed`). A non-null-but-dangling override is _not_
an error; it falls through to the global setting.

### Dangling-reference behavior

Never fail the dictation for a dangling override (00 §8.6). Fall back to the global setting and
show a **one-time HUD notice that names the mode**, so the user learns which mode to fix without
losing the dictation. The notice uses the existing transient-notice channel (4 s auto-clear,
`pipeline.rs::set_transient`).

| Case                                                                          | What resolves                    | HUD notice (exact copy)                                                                                           |
| ----------------------------------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Mode points at a **deleted AI profile** (`aiProfileId` set, file gone)        | Global active profile (or No AI) | `"<Mode>: its AI profile is missing — used your active profile instead."`                                         |
| Mode points at a **deleted STT model** (`sttModelId` set, registry/file gone) | Global speech model              | `"<Mode>: its speech model is missing — used your default model instead."`                                        |
| Mode points at an **STT model not installed** (known model, never downloaded) | Global speech model              | `"<Mode>: its speech model isn’t downloaded — used your default model instead."`                                  |
| Mode language is an **unknown code** (hand-edited JSON)                       | Global language                  | `"<Mode>: its language setting was invalid — used your default."`                                                 |
| **Both** AI profile and STT model dangle                                      | Global for each                  | Show only the first by precedence order (AI profile, then STT, then language); one notice per job, never a stack. |

Notes:

- `<Mode>` is the mode's display name, truncated like the HUD label (§5).
- Dictation still produces output in every row — that is the point. The notice is informational;
  the text already pasted.
- "One-time" means _one notice per job_, not "once ever". A persistently broken override notices
  once each time it is used, which is the gentle nudge to open Settings. We do not persist a
  "already warned" flag (no new persistence, 00 §8.7).
- A no-AI mode (`usesLlm: false`) with a non-null `aiProfileId` never notices: the profile is
  never resolved, so its danglingness is irrelevant (§8, Literal + override).

---

## 4. Per-mode hotkeys

### One-shot, not switch-and-stay

A mode hotkey **dictates once in that mode and leaves the persistent active mode unchanged.**
Press the Notes hotkey → this one dictation uses Notes → the active mode is still whatever it was.

Rejected alternative: _switch the active mode, then dictate._ Concrete confusion: a user on
Standard taps their Email hotkey to fire off one quick email, then later holds the plain
dictation hotkey (`⌥Space`) expecting Standard — and gets Email, because the earlier tap silently
changed the active mode. The mode hotkey would have a side effect that outlives the action the
user took. One-shot has no hidden state: the hotkey _is_ "dictate in this mode", and the tray /
Modes radio remain the only things that move the persistent selection.

One-shot also composes cleanly with the future per-app rules (§9): both are "resolve a mode for
_this_ job without disturbing the active mode."

Mechanically: a mode hotkey starts a `Job::Dictation` carrying an explicit `mode_id` override
(see §6). The resolver in §3 uses that mode instead of `settings.active_mode_id` for this job
only. Everything else — recording, hold/tap latch, generation counter, busy guard — is identical
to the plain dictation hotkey.

### Collision rules

Mode hotkeys join the existing **pairwise-distinct** validation. Today `shortcuts.rs::apply`
checks dictation ≠ rewrite; REFINE.md extends it to all three globals (dictation, rewrite,
polish). Mode v2 extends it to _every_ registered accelerator:

> Every accelerator across the three global hotkeys **and** every mode hotkey must be pairwise
> distinct. Any duplicate makes `apply` fail, and `save_settings` rolls back to the last working
> set.

`save_settings` rollback (REFINE.md already covers the three globals) gains the mode hotkeys: on a
failed `apply`, revert `dictation_hotkey`, `refine_hotkey`, `polish_hotkey`, **and** every
`modes[].hotkey` to the previous settings, re-apply, re-emit `settings-changed`, and return the
error. The mode-hotkey set travels with the globals as one atomic rollback unit, because they are
registered together.

A mode hotkey of `null` registers nothing (the common case). Empty string normalizes to `null`.

### Registration lifecycle

Mode hotkeys register in the same `shortcuts::apply` pass as the globals. `apply` already calls
`unregister_all()` then re-registers from settings; it gains a loop over
`settings.modes.filter(hotkey != null)`, each dispatching `Job::Dictation` with that mode's id.

Re-registration triggers (all already call `save_settings` → `apply`):

- a mode hotkey added, changed, or cleared in the Mode editor;
- a mode **deleted** (its hotkey must be released);
- a mode **duplicated** (the copy starts with `hotkey: null`, so nothing new registers — §5).

Tray-driven active-mode changes do **not** re-register (the active mode is not a hotkey). Only
edits that touch a `modes[].hotkey` value re-register.

### Cap

**Five mode hotkeys.** Past five, two things break: Carbon `RegisterEventHotKey` slots and
modifier space get tight once you add the three globals (the recorder must still offer the user a
free chord), and — more bindingly — a user cannot remember more than a handful of
"hold-this-for-that-mode" gestures before the hotkeys stop saving time and start being a thing to
look up. Five covers "my two or three daily modes plus room to grow"; the tray remains the
unlimited switch surface for the long tail. The cap is enforced in `save_settings`: a sixth
non-null mode hotkey is rejected with
`"At most 5 modes can have their own hotkey. Remove one first."` (rolled back like any hotkey
failure). The number is a soft ergonomic call, not a hard Carbon limit — easy to raise later if
real use proves it low.

---

## 5. Switching UX

Every surface that can change which mode runs, and the feedback it gives:

| Surface                        | Gesture                           | Changes persistent active mode? | Feedback                                                   |
| ------------------------------ | --------------------------------- | ------------------------------- | ---------------------------------------------------------- |
| **Tray radio** (current)       | `tray> Mode > Notes`              | **Yes**                         | Radio dot moves; next HUD label shows the name (below).    |
| **Modes page radio** (current) | click the radio in the Modes list | **Yes**                         | Radio moves; `settings-changed` re-renders; tray rebuilds. |
| **Per-mode hotkey** (new, §4)  | hold the mode's hotkey            | **No** (one-shot)               | HUD label shows that mode's name for this dictation only.  |
| **Per-app rule** (future, §9)  | (automatic on app focus)          | **No**                          | Same one-shot label.                                       |

### The HUD confirmation moment

The switch is confirmed where the user is already looking — the HUD — by putting the mode name in
the listening label. F2 (03 §5) calls for this.

```
Listening — <ModeName>
```

Label spec:

- Shown in the `recording` state of a `Job::Dictation` only. Rewrite and Polish keep their own
  labels (REFINE.md HUD table) — they are actions, not modes, and must never read as a mode.
- The em dash and name are appended to the existing `"Listening…"` base. With no special mode the
  product may keep plain `"Listening…"` for the default Standard mode to avoid noise, or always
  show the name — that micro-decision is 09's (HUD polish); this doc only fixes the _format_.
- **Truncation:** mode names render up to **16 characters**; longer names truncate with an ellipsis
  on a character boundary (`"Quarterly Boa…"`). 16 keeps the pill within its fixed width (00 §8.2 —
  the HUD must not resize). The same truncation applies to `<Mode>` in the §3 dangling notices.

Subsequent states are unchanged: `Transcribing…` / `Refining→(Polishing…)` / `Inserting…` do not
repeat the mode name; the listening label already established it.

### Built-ins stay read-only

The current `ModesTab` rule holds: built-in modes are **read-only**; to change one you **duplicate
to customize**. Overrides are part of that rule — **you cannot set overrides on a built-in mode.**
The Advanced section (§7) is read-only (showing inherited values) for built-ins and editable only
on custom modes. This keeps the four built-ins identical across machines and gives every
"customize" path one shape: duplicate, then edit the copy.

Duplicating a built-in copies its **prompt but not its `builtIn` flag** (existing behavior, kept):
the copy is a custom mode (`builtIn: false`), editable, with all overrides `null` and `hotkey:
null`. It does not inherit the built-in's identity, only its starting prompt.

---

## 6. Settings schema + migration

### Mode v2 (Rust, `settings.rs`)

All additions are nullable and `#[serde(default)]`, so old files load unchanged and a mode with no
overrides serializes/deserializes exactly as a v1 mode does. `Mode` already lives in a struct that
derives `Serialize, Deserialize` with `rename_all = "camelCase"`.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]   // `default` added so partial modes load
pub struct Mode {
    pub id: String,
    pub name: String,
    pub built_in: bool,
    pub uses_llm: bool,
    pub prompt: String,
    // --- Mode v2 overrides, all null = inherit ---
    pub ai_profile_id: Option<String>,   // null = global active AI profile
    pub stt_model_id: Option<String>,    // null = global speech model
    pub language: Option<String>,        // null = global spoken language
    pub hotkey: Option<String>,          // null = no per-mode hotkey
}
```

`built_in_modes()` (modes.rs) sets all four overrides to `None` for Standard / Email / Notes /
Literal — built-ins never carry overrides (§5).

### Settings version: 2 → 3

The Refine work already moved settings to v2 (`activeLlmProfileId`, `refineAfterDictation`,
`polishHotkey`, inline `llm` removed). Mode v2 bumps **v2 → v3**. The migration is a **no-op for
existing modes**: serde `default` fills the four new fields with `None`, and `normalize()` simply
stamps `version = 3`. There is nothing to rewrite — a v2 mode _is_ a valid v3 mode with all
overrides inherited.

| Field                              | Change              |
| ---------------------------------- | ------------------- |
| `version`                          | 2 → 3               |
| `Mode.aiProfileId: Option<String>` | new, default `null` |
| `Mode.sttModelId: Option<String>`  | new, default `null` |
| `Mode.language: Option<String>`    | new, default `null` |
| `Mode.hotkey: Option<String>`      | new, default `null` |

`normalize()` gains two rules (both repair, never reject — matching the existing normalize style):

1. Built-in modes have all four overrides forced to `None` (defends the read-only invariant §5
   against a hand-edited file).
2. An empty-string `hotkey` is coerced to `None` (so "" never reaches the registrar).

The installed-model and profile-existence checks are **not** done in `normalize` — they are
resolution-time concerns (§3), because a model can be deleted after settings are saved and we must
not rewrite the user's chosen override behind their back.

### IPC mirror (`packages/core/src/types.ts`)

Exact new `Mode` interface (the four fields are added; nothing else in the interface changes):

```ts
export interface Mode {
  id: string;
  name: string;
  builtIn: boolean;
  usesLlm: boolean;
  /** System prompt used for LLM refinement. */
  prompt: string;
  /** Mode v2 overrides. null = inherit the global setting. */
  /** AI profile (LLM connection) id, or null to use the active profile. */
  aiProfileId: string | null;
  /** Whisper model id, or null to use the global speech model. */
  sttModelId: string | null;
  /** ISO 639-1 code or `auto`, or null to use the global language. */
  language: string | null;
  /** Accelerator string (e.g. `Alt+Ctrl+N`), or null for no mode hotkey. */
  hotkey: string | null;
}
```

`Settings.version` is documented as `3`. No new commands and no new events: mode edits already
flow through `save_settings` and broadcast `settings-changed`; a mode hotkey is just a
`modes[].hotkey` value saved that way.

### Commands and triggers touched

| Touch point                       | Change                                                                                                                                                                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `save_settings` (commands.rs)     | Hotkey rollback set extended to include every `modes[].hotkey`; new "≤ 5 mode hotkeys" guard; rollback re-applies and re-emits as today.                                                         |
| `shortcuts::apply` (shortcuts.rs) | Loop-register non-null mode hotkeys; pairwise-distinct check spans globals + mode hotkeys.                                                                                                       |
| `pipeline.rs`                     | Resolver (§3) replaces the direct reads of `settings.active_mode_id` / `stt_model_id` / `language`; accepts a per-job `mode_id` override from a mode hotkey; danger notices via `set_transient`. |
| `tray::rebuild_menu`              | Unchanged trigger set; the mode list already rebuilds on `settings-changed`. Mode hotkeys are not shown in the tray.                                                                             |

Tray rebuild triggers stay as they are (mode added/renamed/activated); mode hotkeys do not appear
in the tray, so they add no rebuild trigger.

---

## 7. Wireframes

### Mode editor — Advanced collapsed (default)

```
┌─ Mode ─────────────────────────────────────────────────────────┐
│  Name        Standup notes                                       │
│  Uses AI     [ on ]   Refine this mode’s transcripts with AI.    │
│  Prompt      ┌───────────────────────────────────────────────┐  │
│              │ Turn dictation into short bullet points…       │  │
│              └───────────────────────────────────────────────┘  │
│                                                                  │
│  ▸ Advanced  AI profile · Speech model · Language · Hotkey       │
│                                                                  │
│  [ Duplicate ]  [ Export ]  [ Import ]              [ Delete ]   │
└──────────────────────────────────────────────────────────────────┘
```

### Mode editor — Advanced expanded (custom mode)

Inherit placeholders show the value they inherit, so "inherit" is never a mystery.

```
┌─ Mode ─────────────────────────────────────────────────────────┐
│  Name        Standup notes                                       │
│  Uses AI     [ on ]                                               │
│  Prompt      ┌───────────────────────────────────────────────┐  │
│              │ Turn dictation into short bullet points…       │  │
│              └───────────────────────────────────────────────┘  │
│                                                                  │
│  ▾ Advanced                                                      │
│    AI profile    Inherit — currently Ollama qwen2.5 (local) ▾   │
│    Speech model  Inherit — currently base.en ▾                  │
│    Language      Inherit — currently English ▾                  │
│    Mode hotkey   ⌥ ⌃ N                          [ Record ] [ × ] │
│                  One-shot: dictates in this mode; active mode    │
│                  is unchanged.                                   │
│                                                                  │
│  [ Duplicate ]  [ Export ]  [ Import ]              [ Delete ]   │
└──────────────────────────────────────────────────────────────────┘
```

Each override dropdown's first row is `Inherit — currently <resolved global> ▾`; choosing a
specific value pins the override. Re-selecting "Inherit" sets it back to `null`. For a **built-in**
mode the whole Advanced block is read-only: the same rows render as static text
(`AI profile   Inherit — currently …`) with no controls, and a one-line note: "Duplicate this mode
to change these."

### Tray (modes + refine toggle)

```
┌──────────────────────────────┐
│  Mode                         │   (header, disabled)
│   ○ Standard                  │
│   ● Email                     │   ● = active mode
│   ○ Notes                     │
│   ○ Literal                   │
│   ○ Standup notes             │   (custom modes listed too)
│  ──────────────────────────   │
│   ✓ Refine Dictation with AI  │   (CheckMenuItem; REFINE.md)
│  ──────────────────────────   │
│   Copy Last Result            │
│  ──────────────────────────   │
│   Settings…                   │
│  ──────────────────────────   │
│   Quit Velata               │
└──────────────────────────────┘
```

### HUD label states

```
[ ▮▮▯▯▯ ]  Listening — Notes          (dictation, mode named)
[ ▮▮▮▯▯ ]  Listening for instruction…  (rewrite — never a mode name)
[ ⠿     ]  Transcribing…
[ ⠿     ]  Polishing…                  (dictation refine)
[ ⠿     ]  Inserting…
[  ⓘ    ]  Standup notes: its speech model isn’t downloaded —
           used your default model instead.     (§3 dangling notice, 4 s)
```

---

## 8. Edge cases

- **Mode deleted while active.** Existing fallback holds (`settings.rs::normalize` →
  `active_mode_id` falls back to Standard if it points nowhere). The deleted mode's hotkey is
  released on the next `apply` (§4). Keep this behavior.
- **Mode hotkey pressed mid-pipeline.** Busy guard (00 §5, `pipeline.rs::start` rejects unless
  status is Idle/Error/Notice). A mode hotkey is a `Job::Dictation` like any other and is ignored
  while busy. No new path.
- **Export / import a mode whose overrides point at machine-local ids.** Connections (AI profiles)
  and downloaded models do not travel between machines — their ids are local. On **import**, null
  out `aiProfileId` and `sttModelId` (and `hotkey`, which may collide on the target machine), keep
  `prompt`, `name`, `usesLlm`, and `language` (language is portable), and show a notice:
  `"Imported <Mode>. Its AI profile and speech model were reset — pick them on this machine."`
  This aligns with 06's import rules (06 owns the import/export UI; this is the override-specific
  reset). The imported mode is always a custom mode (`builtIn: false`).
- **Literal (no-AI) mode + an AI-profile override.** `usesLlm: false` wins, unconditionally. The
  AI-profile row in the editor is **disabled** with the explanation: "This mode doesn’t use AI, so
  it has no AI profile." A no-AI mode never resolves a profile (§3), so a stray `aiProfileId` from
  a hand-edited file is inert and never produces a dangling notice. Turning Uses AI back on
  re-enables the row.
- **Override points at the _same_ model/profile as the global.** Harmless and allowed — the
  override simply pins what the global happens to be now, so a later global change won't move this
  mode. This is a feature (Researcher pinning a multilingual model), not a redundancy to collapse.
- **Hand-edited unknown language code.** Resolves to global with the §3 notice; never blocks
  dictation.

---

## 9. Out of scope

- **Auto-switching by app (per-app modes).** Roadmap (00 §9, 03 §4 future table). This design is
  built to slot into it: a future **App rules** card on the Modes page holds a `bundleId → modeId`
  table, and on app focus the pipeline resolves that mode through the **same one-shot path** a
  mode hotkey uses (§4, §6) — no active-mode change, no second resolver. The override resolution in
  §3 is reused verbatim. Nothing here needs to change when per-app modes land; the App rules card
  just becomes another source of a per-job `mode_id`.
- **Profiles for HUD appearance.** A mode bundles _behavior_ (instructions, models, AI, language,
  hotkey), not _chrome_. HUD look (theme, position, size) is global and owned by 09. Bundling
  appearance into modes would multiply the HUD's state space against the never-hide invariant
  (00 §8.2) for no clear user gain. Cut.
- **Per-mode insert method.** **Cut for v2.** Insert method (paste vs clipboard) is an Output-page
  global (03). It is an environment/accessibility property of the user's machine ("can Velata
  paste here?"), not a property of the persona — Developer and Executive both want paste when
  Accessibility is granted and both fall back to clipboard when it isn't. Bundling it per-mode
  would invite the confusing combination "Notes pastes but Email only copies" with no persona
  reason behind it. If real demand appears, it slots in later as a fifth override
  (`insertMethod: Option<InsertMethod>`) under the same resolver — the §3 precedence already
  generalizes — so cutting it now costs nothing later.
- **Persona "profile" objects, accounts, sync, a persona marketplace.** Excluded by D1 (§1) and
  00 §8.8. Personas are templates (06) over the one Mode concept; nothing persists beyond the
  settings file and per-machine profile/model files.

---

Cross-references: 00 (baseline, vocabulary §7, constraints §8), 03 (D1 foundation, sitemap, flows
F2/F3/F10), 06 (prompts, template gallery, import/export UI), 08 (Models page — AI profiles and
STT models the overrides point into), 09 (HUD label polish, appearance). `REFINE.md` (the AI-profile
machinery and the v1→v2 settings migration this builds on).
