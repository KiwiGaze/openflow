# Refine — design

Status: implemented 2026-06-11. Written the same day against `main` (cd63494).

Refine is the umbrella for everything that sends text through the user's LLM:
polishing a selection with one keystroke, rewriting a selection from a spoken
instruction, and the optional cleanup pass after dictation. This document
designs three additions: a no-voice **Polish selection** shortcut, a single
**Refine dictation** toggle, and file-backed **AI profiles** with one active
profile at a time. It builds on what already ships: the voice rewrite hotkey,
the selection capture/insert machinery in `output.rs`, and the single LLM
client in `llm.rs`.

## Requirements → design map

| Requirement                                                                                                                    | Design element                                                                                                                                                      | Section          |
| ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| Shortcut refines the selected text; result replaces the selection, else lands in the focused area, else stays on the clipboard | `⌥⇧P` Polish selection + the existing insert chain, formalized as replace → caret insert → clipboard                                                                | UX, Insertion    |
| Toggle: does transcription include refining afterwards                                                                         | `refineAfterDictation` master switch (settings, Refine tab, tray)                                                                                                   | Dictation toggle |
| Multiple LLM profiles; one selectable at a time; the rest stored as files                                                      | `<app-data>/profiles/*.json`, one file per profile; `activeLlmProfileId` pointer in settings                                                                        | Data             |
| Refinement does not always need a spoken instruction                                                                           | Polish runs with a built-in instruction and no recording at all; Rewrite with an empty utterance falls back to the same instruction (already shipped in `modes.rs`) | Behavior         |
| Two modes, separated by different shortcuts                                                                                    | Polish (`⌥⇧P`, tap) and Rewrite (`⌥⇧Space`, hold and speak)                                                                                                         | UX               |

## UX

### The two selection shortcuts

|                | Polish selection (new)                                                                                               | Rewrite selection (ships today)                                                      |
| -------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Default hotkey | `⌥⇧P`, tap                                                                                                           | `⌥⇧Space`, hold                                                                      |
| Voice          | None. No microphone, no recording.                                                                                   | Hold and speak the instruction; release to apply.                                    |
| Instruction    | Built-in: "Fix grammar, spelling, and clarity. Keep the meaning, tone, and language." (`DEFAULT_REFINE_INSTRUCTION`) | Whatever was spoken; an empty utterance falls back to the same built-in instruction. |
| Use case       | "Make this correct" — fast, predictable, every few minutes.                                                          | "Make it shorter", "translate to German" — deliberate edits.                         |
| Pipeline       | selection → refining → inserting                                                                                     | selection → recording → transcribing → refining → inserting                          |

Both act on the current selection in the frontmost app and end in the same
insertion chain. Polish is a tap, not a hold — there is nothing to record, so
hold semantics, the 350 ms tap latch, and `HotkeyBehavior` do not apply.
`⌥⇧P` is unclaimed by macOS, sits next to the existing `⌥Space` family, and
P reads as "polish". All three hotkeys stay rebindable and must be pairwise
distinct.

Naming hazard: the app already uses "Modes" for dictation styles (Standard,
Email, Notes, Literal). UI copy must never call Polish and Rewrite "modes" —
they are shortcuts/actions. "Mode" keeps meaning dictation style only.

### HUD feedback

`hudLabel` becomes job-aware in the refining state so each flow reads
distinctly at a glance:

| State        | Dictation       | Rewrite                      | Polish                 |
| ------------ | --------------- | ---------------------------- | ---------------------- |
| recording    | "Listening…"    | "Listening for instruction…" | — (state never occurs) |
| transcribing | "Transcribing…" | "Transcribing…"              | —                      |
| refining     | "Polishing…"    | "Rewriting…"                 | "Polishing selection…" |
| inserting    | "Inserting…"    | "Inserting…"                 | "Inserting…"           |

Polish shows "Polishing selection…" immediately on press (set before selection
capture), so feedback is instant even though capture itself takes ~340 ms of
clipboard settle time. `hud::position_on_cursor_monitor` runs on every entry
point, as it does for recording today.

### Settings UI

Principle: every feature is one flat row — a name that states what it does, a
control, at most one short hint line. No nested or indented descriptions; a
user should understand each row without reading anything else.

`General → Hotkeys` gains one row and keeps all bindings in one card:

```
Hotkeys
  Dictation           Hold and speak; release to insert.        ⌥ Space
  Dictation style     How the dictation hotkey behaves.         Hold to talk ▾
  Polish selection    Fix grammar and clarity. No voice.        ⌥ ⇧ P
  Rewrite selection   Hold and say what to change.              ⌥ ⇧ Space
```

The "AI Provider" tab becomes the **Refine** tab (`ProviderTab.tsx` →
`RefineTab.tsx`). Top to bottom: the dictation toggle, the profile list, the
editor for the selected profile.

```
Refine
  Refine dictation with AI                                      [on/off]
  Polish transcripts with the active profile after transcribing.

AI profiles                              [Show in Finder]  [New profile]
  ( ) No AI — rules-based cleanup only
  (•) Ollama qwen2.5        local — qwen2.5:3b
  ( ) Groq Llama            cloud — llama-3.3-70b

Edit profile                                    [Test connection] [Delete]
  Name        Groq Llama
  Provider    OpenAI-compatible API ▾
  Base URL    https://api.groq.com/openai/v1
  API key     ••••••••••••
  Model       llama-3.3-70b-versatile
  Timeout     30 s
```

The list reuses the radio-selects-active idiom from the speech-model list and
the modes list: the radio is the single "used for refinement" selector, and
"No AI" is a first-class choice (`activeLlmProfileId = ""`), replacing today's
`provider: none`. Clicking a row selects it for editing without changing the
radio, exactly like `ModesTab`. Each row's badge says `local` or `cloud`
derived from the base-URL host (localhost ⇒ local), not from the provider
kind — LM Studio and llama.cpp are OpenAI-compatible but local, and the badge
exists so the privacy story is visible at a glance. The cloud privacy note
("refined text — never audio — is sent to this endpoint") moves into the
editor and shows only for non-local URLs.

"Show in Finder" reveals `<app-data>/profiles/` via the opener plugin. That is
the import/export story: profiles are plain JSON files; drop one in and it
appears the next time the list loads. No dedicated import UI.

With no profiles and the radio on "No AI", the tab shows one empty-state line:
"Dictation uses fast rules-based cleanup. Add a profile to enable AI polish
and the selection shortcuts." — plus the New profile button.

### Tray

One `CheckMenuItem` under the mode list: **Refine Dictation with AI**, bound
to `refineAfterDictation`. It mirrors the tray's existing role as the
quick-switch surface (modes live there already). No tray item triggers Polish
or Rewrite: clicking the menu bar changes the frontmost app, so the synthetic
⌘C/⌘V would target the wrong window. The hotkeys are the only entry points
for selection jobs.

## Behavior

### Polish flow

```
⌥⇧P pressed
→ busy guard (status must be idle / error / notice; otherwise ignore)
→ active profile resolved; none → error notice
   "polishing needs an AI profile — add one in Settings"
→ state Refining(polishSelection), HUD positioned
→ capture_selection() via ⌘C probe (output worker; clipboard restored)
   no selection → error notice "select some text first…"
→ llm.chat(selection_system_prompt, selection_user_prompt(selection, ""))
→ generation re-check → insert (chain below) → idle
```

Polish runs under the same generation counter and busy-state contract as every
other job, but outside the `Session`/`finish()` machinery — there is no
recording to stop. `cancel()` keeps working unchanged: bumping the generation
orphans any in-flight polish before it can insert. A new `Job::PolishSelection`
(`'polishSelection'` on the wire) identifies it in pipeline state.

### Rewrite flow

Unchanged, except the provider check becomes an active-profile check and the
chat call uses the active profile. The empty-instruction fallback in
`selection_user_prompt` stays as is — it is the second half of "refinement
does not always require an instruction".

### Dictation and the toggle

`refineAfterDictation` is the master switch for the dictation → LLM handoff.
`finish_dictation` gates on it in addition to the existing conditions:

| Mode `usesLlm` | Toggle | Active profile | Result                                                  |
| -------------- | ------ | -------------- | ------------------------------------------------------- |
| no (Literal)   | —      | —              | literal text (dictionary still applies)                 |
| yes            | off    | —              | rules-based cleanup; no network call, guaranteed        |
| yes            | on     | none           | rules-based cleanup (today's `provider: none` behavior) |
| yes            | on     | set            | LLM refine; on provider error → rules cleanup + notice  |

The toggle ANDs with per-mode `usesLlm` rather than replacing it: modes say
what shape of output they want, the toggle says whether the LLM may touch
dictation at all right now. Off is an unconditional kill switch — useful when
the provider is slow, metered, or the user is dictating sensitive content.
Default is **on**, which changes nothing: fresh installs have no profile (so
nothing fires), and upgraders configured their provider deliberately.

Polish and Rewrite ignore the toggle. They are explicit per-invocation AI
requests; an "AI off" switch that also disabled the AI-only shortcuts would
just make them dead keys.

### Insertion: replace → append → clipboard

Both selection jobs and dictation end in `output::insert`. The contract,
formalized:

| Step         | Condition                                                                        | Mechanism                                                | User sees                                              |
| ------------ | -------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------ |
| 1. Replace   | Selection still active in the frontmost app                                      | synthetic ⌘V pastes over it                              | selection replaced in place                            |
| 2. Append    | Selection collapsed meanwhile (click, arrow key)                                 | the same ⌘V inserts at the caret of the focused area     | result inserted at the cursor; original text untouched |
| 3. Clipboard | No Accessibility, insert method set to clipboard-only, or the ⌘V keystroke fails | result stays on the clipboard, clipboard restore skipped | notice: "Copied to clipboard — press ⌘V to paste"      |

Steps 1 and 2 are both native ⌘V semantics — no AX-tree inspection, no
per-app special cases, and no way to lose the user's text between them.

One behavior fix rides along: today, if the ⌘V dispatch itself fails (main
thread busy, keystroke error) after the clipboard was already written,
`insert()` returns an error — the HUD shows a failure even though the result
is sitting on the clipboard. The design changes `InsertOutcome` to
`Pasted | CopiedToClipboard(reason)` with `reason ∈ {chosenMethod,
noAccessibility, pasteFailed}`; a failed paste degrades to the clipboard
outcome (restore skipped) and the pipeline picks the notice text by reason.
This upholds the existing rule: dictation output is never silently dropped —
worst case it lands on the clipboard, and the HUD says so.

### Failure policy

Dictation keeps its fallback (LLM error → rules cleanup + notice). Selection
jobs keep their deliberate no-fallback: on any LLM error the HUD shows the
error, nothing is inserted, and the selection is untouched — replacing the
user's text with something other than what they asked for is worse than doing
nothing. Polish inherits that rule.

## Data and persistence

### Profile files

One profile per file under `<app-data>/profiles/` (sibling of `models/`),
camelCase, schema-versioned, written atomically (temp + rename) like
`settings.json`, created with mode 0600 — profiles can hold API keys.

```jsonc
// <app-data>/profiles/3f2a….json
{
  "version": 1,
  "id": "3f2a…", // always equals the filename stem
  "name": "Groq Llama",
  "provider": "openaiCompatible", // "ollama" | "openaiCompatible"
  "baseUrl": "https://api.groq.com/openai/v1",
  "apiKey": "gsk_…",
  "model": "llama-3.3-70b-versatile",
  "timeoutSecs": 30,
}
```

Rules: the filename stem is the identity; the in-file `id` is normalized to it
on load (covers hand-copied files). Files that fail to parse are skipped with
a log warning and never deleted — they are the user's files. There is no
`none` provider in a profile; "no AI" is the absence of an active profile.
The directory is scanned at startup, on every `list_llm_profiles` call, and
after each mutation; the pipeline reads the active profile from an in-memory
`RwLock` cache, mirroring `SettingsManager`.

A new `profiles.rs` owns this: `LlmProfile`, `LlmProviderKind` (moves here,
drops `None`), and `ProfileManager { list, get, save, delete, active }`.
`llm.rs::chat`/`test` take `&LlmProfile` instead of `&LlmConfig`;
`LlmConfig` is deleted. Deleting the active profile clears
`activeLlmProfileId` (refinement turns off until another is chosen); no
confirmation dialog, matching `ModesTab`.

### settings.json v2

| Field                           | Change                        |
| ------------------------------- | ----------------------------- |
| `version`                       | 1 → 2                         |
| `llm: LlmConfig`                | removed (migrated, see below) |
| `polishHotkey: string`          | new, default `"Alt+Shift+P"`  |
| `refineAfterDictation: boolean` | new, default `true`           |
| `activeLlmProfileId: string`    | new, default `""` (= No AI)   |

`normalize()` gains one rule: if `activeLlmProfileId` points at a profile that
does not exist or does not parse, clear it to `""`.

### Migration v1 → v2

`Settings` keeps a deserialize-only legacy field
(`#[serde(default, skip_serializing)] llm: Option<LlmConfig>`). In
`main.rs::setup`, after `SettingsManager::load` and `ProfileManager::new`: if
the legacy block is present with `provider != none`, write it as
`profiles/<uuid>.json` named after its provider and model (e.g. "Ollama —
qwen2.5:3b"), set `activeLlmProfileId`, and persist settings — which drops the
legacy field from disk, making the migration self-erasing and idempotent.
`provider == none` migrates to `activeLlmProfileId = ""` with no file. Nothing
else changes for the user: same endpoint, same model, now visible as a named
profile.

## IPC contract changes

Both sides in the same PR, as always: Rust structs ↔ `packages/core/src/types.ts`.

Types: `PipelineJob` += `'polishSelection'`. `LlmConfig` is replaced by
`LlmProfile` (shape above). `LlmProviderKind` = `'ollama' | 'openaiCompatible'`.
`Settings` per the v2 table. `TranscriptionResult` is unchanged; polish
results use `modeId: "polish"`, `raw` = the original selection (in-memory
only, like every result — nothing is persisted).

| Command                                      | Change                                                                                                                    |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `start_polish_selection`                     | new; spawn-blocking like `start_refine_selection` (selection capture round-trips the main thread — inline would deadlock) |
| `list_llm_profiles` → `LlmProfile[]`         | new; rescans the directory                                                                                                |
| `save_llm_profile(profile)` → `LlmProfile[]` | new; upsert, returns the fresh list                                                                                       |
| `delete_llm_profile(id)` → `LlmProfile[]`    | new                                                                                                                       |
| `reveal_llm_profiles`                        | new; opener `reveal_item_in_dir`                                                                                          |
| `test_llm(profile: LlmProfile)`              | signature change                                                                                                          |
| `save_settings`                              | hotkey re-registration and rollback now cover all three hotkeys                                                           |

Active-profile selection is just `save_settings` with a new
`activeLlmProfileId` — no dedicated command, and the existing
`settings-changed` event already broadcasts it. Profile mutations return the
updated list instead of adding a new event.

## Implementation map

| File                                                | Change                                                                                                                                  |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `profiles.rs` (new)                                 | `LlmProfile`, `LlmProviderKind`, `ProfileManager`                                                                                       |
| `settings.rs`                                       | v2 fields, legacy `llm` migration field, normalize rule                                                                                 |
| `llm.rs`                                            | `chat`/`test`/`connect_hint` take `&LlmProfile`; `LlmConfig` deleted                                                                    |
| `pipeline.rs`                                       | `Job::PolishSelection`, `polish()` entry point, toggle gate in `finish_dictation`, active-profile lookups, notice-by-reason in `insert` |
| `output.rs`                                         | `InsertOutcome` reasons; failed ⌘V degrades to clipboard outcome                                                                        |
| `shortcuts.rs`                                      | third registration, 3-way distinct check, polish dispatch (Pressed only; Released is a no-op)                                           |
| `commands.rs`                                       | new commands; 3-hotkey rollback in `save_settings`                                                                                      |
| `state.rs`, `main.rs`                               | `profiles: Arc<ProfileManager>`; migration call; handler list                                                                           |
| `tray.rs`                                           | Refine Dictation check item                                                                                                             |
| `modes.rs`                                          | unchanged — polish reuses `selection_system_prompt` and the empty-instruction fallback                                                  |
| `types.ts`, `ipc.ts`, `hooks.ts`                    | mirror types, command wrappers, a profiles hook                                                                                         |
| `App.tsx`, `RefineTab.tsx` (from `ProviderTab.tsx`) | tab renamed to Refine; toggle + list + editor                                                                                           |
| `GeneralTab.tsx`                                    | Polish selection hotkey row                                                                                                             |
| `hudState.ts`                                       | job-aware refining labels                                                                                                               |
| `validate.ts`                                       | `isLocalEndpoint` for the local/cloud badge                                                                                             |
| `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT.md`       | persistence section; manual-test checklist                                                                                              |

Threading is untouched: selection capture and insertion stay on the output
worker with keystrokes marshaled to the main thread; the LLM call is plain
async; polish never touches the audio thread or whisper at all.

## Edge cases

- Polish or Rewrite with nothing selected → error notice "select some text first…" (existing probe path).
- Press during a busy pipeline → ignored, as today.
- No Accessibility → selection capture fails with the existing grant hint; both shortcuts need it and say so.
- Selection collapses while the LLM runs → step-2 caret insert; nothing lost.
- Provider error or timeout on a selection job → error notice, selection intact, nothing inserted.
- Active profile file deleted or corrupted on disk → `normalize` clears the pointer; next press explains "add an AI profile in Settings".
- Hotkey collision among the three → `save_settings` rejects and rolls back to the last working set.
- Hand-dropped profile JSON → listed on next scan under its filename stem; corrupt files skipped, logged, never deleted.
- Huge selection → bounded by the profile's `timeoutSecs`; a timeout surfaces as an error with the selection untouched.

## Test plan

Rust: profile round-trip, atomic write, corrupt-file skip, id-equals-stem
normalization; v1→v2 migration with `provider: none` and a real provider,
including idempotence; normalize clears a dangling active id; camelCase
contract for the new fields; `InsertOutcome` reason mapping including the
failed-⌘V degrade. TS: type mirror compiles; `hudLabel` for
`polishSelection`; `isLocalEndpoint`; default `polishHotkey` passes
`isValidAccelerator`. Manual (checklist additions in DEVELOPMENT.md): polish
in TextEdit / Safari / Slack; collapse the selection mid-refine and verify
caret insert; revoke Accessibility and verify the clipboard notice; toggle
off and verify no request reaches a local server's logs.

## Privacy

Nothing changes by default. Fresh installs have no profile, so no refine
feature can make a network call; every flow that sends text to an endpoint
exists only because the user created and selected a profile (BYO key). Only
text is ever sent — never audio. Profile files live in the local app-data
directory with 0600 permissions; results stay in memory. Moving API keys to
the Keychain remains the separate roadmap item and slots in later as a
key-reference field in the same profile schema.

## Out of scope

Keychain storage (roadmap), custom polish prompts per profile, per-app
profile auto-switching (pairs with the roadmap's per-app modes), a dedicated
import/export UI beyond Show in Finder, and streaming refinement.
