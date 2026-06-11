# 04 — Onboarding redesign: first dictation in under 60 seconds

Status: design proposal. Written 2026-06-11 against `main` (cd63494) + the in-flight Refine
work (`docs/REFINE.md`). Baseline facts and vocabulary: `00-current-state.md`. Hands off to the
v2 structure in `03-information-architecture.md` (flows F1, F12). The audit (`02-ux-audit.md`)
did not exist when this was written, so no UX-xx findings are cited; cross-reference it later.

## 1. Goal and the one number that matters

A first-time user reaches a **real, successful dictation** in under 60 s of active time. The
model download runs in parallel, off the critical path. Defaults leave the product fully
working with zero configuration: Standard mode, no AI profile, `base.en` (00 §6 defaults).

The success moment is not a claim, it is a demonstration: the user dictates one sentence and
sees the **raw transcript** beside the **cleaned text**, then is pointed at the menu-bar icon so
the app does not get lost (menu-bar apps with no Dock icon vanish — 00 §2).

Five new steps, renamed and re-sequenced from today's five:

```
1 Welcome      value + privacy + "Download base.en" consent (download starts here)
2 Microphone   the one required permission
3 Accessibility paste vs clipboard-only — both framed as working outcomes
4 Try it       a REAL dictation into an in-window field; raw → cleaned diff
5 You're set   where OpenFlow lives + the three things to learn next
```

The change in shape: the speech-model **picker** (today's gated step 4) becomes a single
consent button on step 1, and model _readiness_ is awaited inside step 4 instead of blocking a
Continue button. Nothing serializes behind the download anymore.

## 2. Journey map

| #   | Stage               | User state                    | System state                                     | Screen        | Exit criteria                                     | Active-time budget |
| --- | ------------------- | ----------------------------- | ------------------------------------------------ | ------------- | ------------------------------------------------- | ------------------ |
| 1   | Launch & value      | curious, unsure it is safe    | first run, `onboardingCompleted:false`, no model | Welcome       | reads value, taps "Download base.en"              | 10 s               |
| 2   | Microphone          | willing                       | mic `undetermined`; download running             | Microphone    | mic `granted` (or chooses to continue denied)     | 8 s                |
| 3   | Accessibility       | weighing convenience vs trust | AX `false`; download running                     | Accessibility | grants AX, or accepts clipboard-only              | 8 s                |
| 4   | Try it              | proving it works              | model installs mid-step; pipeline idle           | Try it        | one dictation lands in the field, diff shown      | 20 s               |
| 5   | You're set          | convinced                     | configured, working                              | You're set    | reads the three tips, taps "Start using OpenFlow" | 10 s               |
| —   | Habitual use, day 1 | reaching for ⌥Space by reflex | window closed, tray live                         | —             | dictates in a real app without the window         | n/a                |

Active time excludes download wait: steps 2–3 are where the 148 MB `base.en` finishes on most
connections, so step 4 rarely waits.

### Time-to-value

`base.en` is 148 MB. Active time = steps the user drives; download overlaps steps 2–4.

| Scenario            | Network                   | Download finishes by | Time to first successful dictation                                                                         |
| ------------------- | ------------------------- | -------------------- | ---------------------------------------------------------------------------------------------------------- |
| Best case           | fast (≥50 Mbps)           | during step 2        | ~40 s active, no wait                                                                                      |
| Typical             | home broadband (~20 Mbps) | during step 4        | ~55 s active, ≤10 s wait in step 4                                                                         |
| Worst case (online) | slow (~3 Mbps)            | after step 4 starts  | steps 1–3 unblocked; step 4 shows a download bar, finishes then succeeds (~90 s wall, ~45 s active)        |
| Offline             | none                      | never                | steps 1–3 complete; step 4 states "Connect to the internet to finish the download" with Retry; no dead end |

## 3. Critical-path analysis of the current flow

What is wrong today (`Onboarding.tsx`, 00 §4):

- **The model download gates Continue (step 4).** `disabled={step === 3 && !selectedModel?.installed}`
  serializes the whole flow behind a 148 MB transfer. On slow networks the user stares at a
  disabled button. This is the single biggest time-to-value defect — fixed by moving consent to
  step 1 and awaiting readiness inside "Try it", never on a navigation control.
- **Three competing model choices up front.** base.en / small.en / large-v3-turbo, each with a
  radio and a Download button, force a decision the first-timer cannot make. We pick `base.en`
  for them (it is already the default) and demote the others to Settings.
- **Taught vs never taught.** Today step 5 teaches only hold-to-talk. Never mentioned anywhere:
  the **rewrite hotkey** (⌥⇧Space), **tap-to-latch** hands-free (a tap < 350 ms), **modes**,
  the **dictionary**, **AI refinement**. New flow teaches exactly three at the success moment
  (tap-latch, rewrite, where modes live); the rest is `05-discoverability`'s job (§5, §8).
- **Dead ends.** Mic denied → only "Open System Settings", no recovery copy. Download failure
  → silent (no error surfaced in the model row beyond a missing badge). Skip with nothing
  installed → lands in a non-working app with no explanation. No way to re-run — `onboardingCompleted`
  flips `true` and the only reset is hand-editing JSON. New flow closes all four (§5, §6).
- **"Try it" never proves anything.** It tells the user to click into another app, then shows
  `lastResult.text` only — no raw transcript, so the cleanup value is invisible, and if the
  user never leaves the window nothing happens. New flow makes the success real and in-window.

## 4. Screen-by-screen specification

Window: the existing onboarding view (`App.tsx` renders `<Onboarding>` while
`onboardingCompleted` is false). Step dots stay (`STEPS` array). "Skip setup" stays bottom-left
but gains an honest final-state line (§4.6). Buttons: primary = filled, secondary = outline,
quiet = text (existing `btn` / `btn-primary` / `btn-quiet` classes).

### 4.1 Step 1 — Welcome (value + download consent)

The privacy constraint (00 §8.1: no default-on network) means the download must be an explicit,
visible user action. So consent lives here as a real button, not an automatic fetch. Hugging
Face is one of the two sanctioned calls; the button names the host and the size.

```
┌──────────────────────────────────────────────────────────────┐
│  ● Welcome   ○ Microphone   ○ Accessibility   ○ Try it   ○ Done │
│                                                                │
│   Welcome to OpenFlow                                          │
│                                                                │
│   Hold ⌥Space, speak, release — clean text lands in whatever  │
│   app you're using.                                            │
│                                                                │
│   • Your voice is transcribed on this Mac. Audio never leaves. │
│   • No account, no telemetry, no cloud by default.             │
│   • Optional AI polish via Ollama or your own API key.         │
│                                                                │
│   To transcribe, OpenFlow needs a speech model (148 MB, one    │
│   time). It downloads from Hugging Face, then runs offline.    │
│                                                                │
│        ┌────────────────────────────┐                         │
│        │  Download Base (English)   │   Choose another model ▾ │
│        └────────────────────────────┘                         │
│                                                                │
│  Skip setup                                       [ Continue ] │
└──────────────────────────────────────────────────────────────┘
```

- **Headline:** "Welcome to OpenFlow"
- **Body:** "Hold ⌥Space, speak, release — clean text lands in whatever app you're using."
  (the literal hotkey comes from `formatAcceleratorMac(settings.dictationHotkey)`).
- **Privacy bullets:** as drawn above (kept close to today's, tightened).
- **Consent line:** "To transcribe, OpenFlow needs a speech model (148 MB, one time). It
  downloads from Hugging Face, then runs offline."
- **Primary button:** "Download Base (English)".
  - States: idle → on click calls `download('base.en')`, button becomes a quiet progress chip
    "Downloading… 38%" (from `progress['base.en']`); on `done` it becomes "Base (English) ready
    ✓". `base.en` is already `settings.sttModelId` by default, so no radio is needed.
  - If `base.en` is already installed (re-run case): button is absent; show "Base (English)
    ready ✓".
- **"Choose another model ▾":** a quiet disclosure. Expands an inline list of the three starter
  models (base.en / small.en / large-v3-turbo-q5_0) with the same size + one-line description
  and a per-row Download, identical idiom to Settings → Models (03 §2). Selecting one sets
  `sttModelId` and starts its download. Collapsed by default so the first-timer sees one choice.
- **Continue:** always enabled. Advancing does **not** require the download to be finished — it
  keeps running in the background (`ModelManager` streams to `<file>.bin.part` and renames on
  completion, so a half-file is never mistaken for installed — `models.rs`).
- **Skip setup:** allowed; see §4.6 for the honest final state.
- **Background:** if the user tapped Download, the transfer is now in flight and survives every
  subsequent step.

### 4.2 Step 2 — Microphone

```
┌──────────────────────────────────────────────────────────────┐
│  ○ Welcome   ● Microphone   ○ Accessibility   ○ Try it   ○ Done │
│                                                                │
│   Let OpenFlow hear you                                        │
│                                                                │
│   OpenFlow records only while you hold the hotkey. Audio is    │
│   never written to disk.                                       │
│                                                                │
│   Microphone:  [ undetermined ]                               │
│                                                                │
│        ┌────────────────────────┐                             │
│        │  Allow microphone      │                             │
│        └────────────────────────┘                             │
│                                                                │
│   Base (English): downloading… 61%                            │
│                                                                │
│  Skip setup                              [ Back ]  [ Continue ]│
└──────────────────────────────────────────────────────────────┘
```

- **Headline:** "Let OpenFlow hear you"
- **Body:** "OpenFlow records only while you hold the hotkey. Audio is never written to disk."
- **Status badge:** reflects `permissions.microphone` (`granted` / `denied` / `undetermined` /
  `unknown`), polled every 1.5 s by `usePermissions`.
- **Dependency-detection behavior:**
  - `undetermined` → primary button "Allow microphone" → `requestMicrophonePermission()` fires
    the system prompt (only works while undetermined — `permissions.rs`).
  - `granted` → badge turns green "granted ✓", button hidden, hint "You're set for microphone."
  - `denied` → button becomes "Open System Settings" → `openMicrophoneSettings()`; copy: "macOS
    is blocking the microphone. Open System Settings → Privacy & Security → Microphone, switch
    OpenFlow on, then come back — this updates automatically." (the poll flips the badge when
    they return; no relaunch needed).
- **Dev-mode hint** (kept): "Running from a dev build? The permission attaches to your terminal
  app." Shown small.
- **Download footer:** one quiet line mirroring `base.en` progress, so the parallel work is
  visible but not loud.
- **Advance:** Continue always enabled. Microphone is genuinely required to dictate, but we do
  **not** hard-block here — a user who denied can still proceed, reach "Try it", and be told
  precisely what to fix (a hard block with no escape is the worse dead end). Continue is the
  honest path; the consequence surfaces where it bites.

### 4.3 Step 3 — Accessibility (both outcomes work)

```
┌──────────────────────────────────────────────────────────────┐
│  ○ Welcome  ○ Microphone  ● Accessibility  ○ Try it  ○ Done    │
│                                                                │
│   Paste straight into your apps (optional)                     │
│                                                                │
│   With Accessibility on, OpenFlow types your text into the     │
│   active app for you. Without it, OpenFlow copies the text to  │
│   your clipboard and you press ⌘V — that works too.            │
│                                                                │
│   Accessibility:  [ not granted ]                             │
│                                                                │
│   ┌────────────────────┐  ┌────────────────────────┐          │
│   │  Grant access      │  │  Open System Settings  │          │
│   └────────────────────┘  └────────────────────────┘          │
│                                                                │
│   Skip → OpenFlow uses the clipboard. You can turn this on     │
│   later in System Settings.                                    │
│                                                                │
│  Skip setup                              [ Back ]  [ Continue ]│
└──────────────────────────────────────────────────────────────┘
```

- **Headline:** "Paste straight into your apps (optional)" — the word "optional" sets the frame:
  clipboard-only is a working outcome, not a failure.
- **Body:** "With Accessibility on, OpenFlow types your text into the active app for you. Without
  it, OpenFlow copies the text to your clipboard and you press ⌘V — that works too."
- **Status badge:** `permissions.accessibility` → "granted ✓" / "not granted".
- **Dependency-detection behavior:**
  - not granted → "Grant access" calls `promptAccessibilityPermission()` (shows the macOS dialog
    that deep-links to System Settings) and "Open System Settings" calls
    `openAccessibilitySettings()`. The AX trust check is live-polled, so flipping the toggle
    updates the badge without relaunch.
  - granted → badge green, buttons hidden, hint "OpenFlow will paste for you."
- **Honest skip line:** "Skip → OpenFlow uses the clipboard. You can turn this on later in System
  Settings." No scary language.
- **Advance:** Continue always enabled, regardless of AX state. Step 4 adapts to whichever
  outcome is live.

### 4.4 Step 4 — Try it (a real dictation, in-window)

**Decision: an in-window test field, not "click into another app." Justification:** the paste
path simulates ⌘V into whatever holds keyboard focus (`output.rs` `press_cmd_shortcut`). The
onboarding window holds focus, so a normal text field inside it receives the synthetic paste
exactly like any other app — a genuine end-to-end dictation, not a simulation. It needs **no new
IPC**: a normal `<input>`/`<textarea>` plus the existing `start_dictation` / `stop_dictation`.
"Click into another app" reintroduces the today-flow failure where nothing happens if the user
stays in the window, and it cannot show the raw→cleaned diff in place. The in-window field also
makes the **clipboard-only** outcome demonstrable: when AX is off, `insert()` returns
`CopiedToClipboard` and we prompt an explicit ⌘V into the same field.

```
┌──────────────────────────────────────────────────────────────┐
│  ○ Welcome  ○ Microphone  ○ Accessibility  ● Try it  ○ Done    │
│                                                                │
│   Try your first dictation                                    │
│                                                                │
│   Click the box below, then hold ⌥Space and say:              │
│   "hey open flow this is my first note"                       │
│                                                                │
│   ┌────────────────────────────────────────────────────────┐ │
│   │  (your dictation appears here)                          │ │
│   │                                                         │ │
│   └────────────────────────────────────────────────────────┘ │
│                                                                │
│   ◉ Listening…   ▮▮▮▯▯                                        │
│                                                                │
│  Skip setup                              [ Back ]  [ Continue ]│
└──────────────────────────────────────────────────────────────┘
```

States of this screen, driven by `usePipeline().state.status` and `lastResult`:

- **Model not ready yet** (download still running): the field is disabled with the line
  "Getting the speech model ready… 72%" (from `progress['base.en']`). The user cannot fail by
  trying too early. When `installed` flips true, the field enables and the prompt appears.
- **Idle, ready:** field enabled, focused; live hotkey shown via `formatAcceleratorMac`.
- **recording:** label "Listening…" with the existing level bars (HUD reuses the same
  `audio-level` event; here a small inline meter).
- **transcribing:** "Transcribing…".
- **inserting / done:** the text appears in the field (pasted if AX granted) and the success
  panel slides in (§4.5). `lastResult` carries `raw`, `text`, and `refined` already — no new
  command.
- **Clipboard-only outcome (AX off):** after the pipeline finishes, the field stays empty and a
  line appears: "Copied to your clipboard. Click the box and press ⌘V to drop it in." This is
  shown as a working result, matching the dictation failure policy (00 §5: never lose output).
- **Silence / "Didn't catch anything":** the pipeline notice surfaces inline: "Didn't catch
  anything — hold ⌥Space, speak, then release. Try again." Field stays ready.
- **Mic denied (carried from step 2):** field disabled, line "Microphone is off, so there's
  nothing to transcribe. Turn it on in System Settings → Microphone, then try again." with a
  button to `openMicrophoneSettings()`. The poll re-enables the field on return.
- **Advance:** Continue is enabled once `lastResult` exists for this session (a real success) —
  but is also always clickable so a user who cannot get audio working is never trapped; in that
  case the final step states what is missing (§4.6).

### 4.5 Step 5 — You're set (success moment + teach exactly three)

This screen is the payoff. It shows the diff (proves cleanup), points at the menu bar (so the
app is findable), and teaches three things — one line each — then gets out of the way.

```
┌──────────────────────────────────────────────────────────────┐
│  ○ Welcome  ○ Microphone  ○ Accessibility  ○ Try it  ● Done    │
│                                                                │
│   That's dictation.                          ↑ OpenFlow lives  │
│                                                in your menu bar │
│   You said            →   OpenFlow wrote                       │
│   ┌──────────────────┐    ┌──────────────────────────────┐    │
│   │ hey open flow    │    │ Hey, OpenFlow. This is my     │    │
│   │ this is my first │    │ first note.                   │    │
│   │ note             │    │                               │    │
│   └──────────────────┘    └──────────────────────────────┘    │
│   raw transcript          cleaned with Standard mode          │
│                                                                │
│   Three things worth knowing:                                 │
│   • Tap ⌥Space (don't hold) to keep recording hands-free.     │
│   • Select text, hold ⌥⇧Space, and speak an edit to rewrite it.│
│   • Switch the writing style anytime from the menu-bar Mode list.│
│                                                                │
│  Skip setup                       [ Start using OpenFlow ]     │
└──────────────────────────────────────────────────────────────┘
```

- **Headline:** "That's dictation."
- **Diff panel:** two columns, "You said" (= `lastResult.raw`) → "OpenFlow wrote" (=
  `lastResult.text`). Captions: "raw transcript" and, when `lastResult.refined` is false (the
  zero-config default — Standard mode with no AI profile uses rules cleanup), "cleaned with
  Standard mode"; if `refined` is true, "polished with AI". If the user skipped step 4 and there
  is no `lastResult`, this panel is replaced by a single line: "Hold ⌥Space in any app to
  dictate." (no fake example).
- **Menu-bar pointer:** an arrow/caption at the top-right — "↑ OpenFlow lives in your menu bar"
  — because there is no Dock icon (00 §2) and this is the one place to say so.
- **The three tips (exact copy, one line each):**
  - "Tap ⌥Space (don't hold) to keep recording hands-free." — tap-to-latch (00 §1).
  - "Select text, hold ⌥⇧Space, and speak an edit to rewrite it." — rewrite hotkey (vocabulary:
    "rewrite selection", 00 §7).
  - "Switch the writing style anytime from the menu-bar Mode list." — where modes live.
- **Explicit handoff:** everything else (dictionary, AI profiles, Polish selection, templates,
  per-mode hotkeys, accuracy upgrades) is **not** taught here. It belongs to
  `05-discoverability` (empty-state nudges, the Last-result "Add correction" quick win, contextual
  hints) and to the Settings pages in 03. This screen deliberately stops at three.
- **Primary button:** "Start using OpenFlow" → `update({ onboardingCompleted: true })`, window
  drops to the normal Settings shell (or closes, per current behavior).

### 4.6 Skip — honest final state

"Skip setup" stays available on every step. It must not silently land the user in a broken app.
On click it does **not** immediately set `onboardingCompleted`; it shows a one-screen summary
computed from live state (`permissions`, `models`, `progress`):

```
┌──────────────────────────────────────────────────────────────┐
│   Skipping setup                                              │
│                                                                │
│   Here's where things stand:                                  │
│   ✓ Microphone: granted                                       │
│   ✗ Speech model: not downloaded — dictation won't work until │
│       you download one in Settings → Models.                  │
│   • Accessibility: off — text will go to your clipboard (⌘V). │
│                                                                │
│   You can run this tour again anytime:                        │
│   Settings → General → Welcome tour → Show again.             │
│                                                                │
│              [ Back to setup ]      [ Skip anyway ]           │
└──────────────────────────────────────────────────────────────┘
```

- Each line is computed: ✓ when satisfied, ✗ when it blocks dictation (no model installed and
  none downloading), • when it is a working-but-degraded choice (AX off → clipboard).
- The recovery path is named explicitly: **Settings → General → Welcome tour → Show again**
  (IA F12, 03 §5), which writes `onboardingCompleted: false`.
- "Skip anyway" sets `onboardingCompleted: true`. "Back to setup" returns to the last step.

## 5. Failure-path matrix

| Failure                   | What the user sees                                                                                 | Recovery in-flow                                                                                | What persists                                                                      |
| ------------------------- | -------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Mic denied                | Step 2 badge "denied" + Settings deep link; Step 4 field disabled with the same fix                | Flip toggle in System Settings → poll re-enables (no relaunch); or finish and fix later         | nothing about mic; settings unchanged                                              |
| Download fails / rejected | Consent/footer chip shows "Download failed — Retry" (`progress['base.en'].error` from `models.rs`) | "Retry" calls `download('base.en')` again; `.bin.part` was cleaned up so no half-file           | partial file already removed by `download_inner`                                   |
| Download stalls           | Progress chip stops moving; same Retry available; user can also continue and let step 4 wait       | cancel + retry; or wait                                                                         | `.bin.part` until success or cancel                                                |
| No network / offline      | Step 4: "Connect to the internet to finish the download." + Retry; steps 1–3 still complete        | reconnect → Retry; tour can be re-run later (F12)                                               | nothing                                                                            |
| Accessibility skipped     | Step 3 framed as optional; step 4 demonstrates clipboard-only with an explicit ⌘V prompt           | turn on later in System Settings; tray Copy Last Result and Output → Last result remain (03 F9) | `insertMethod` stays `paste`; AX off only changes runtime outcome                  |
| User quits mid-flow       | Next launch re-enters onboarding at step 1 (because `onboardingCompleted` is still false)          | resume from the top; any model already downloaded shows "ready ✓"                               | `onboardingCompleted:false`; downloaded model file persists; `sttModelId` persists |

**`onboardingCompleted` semantics.** It is the _only_ completion signal and it is **not**
telemetry — it is local state gating which view `App.tsx` renders. It flips to `true` exactly
once, on "Start using OpenFlow" or "Skip anyway". It is never written on intermediate steps, so a
crash or quit always resumes onboarding. F12 sets it back to `false` to re-run. No timestamp, no
counter, no "completed N steps" record — there is nothing to measure (§7).

## 6. Implementation notes

**Reuses existing IPC (00 §6, `types.ts` / `ipc.ts`) — no new commands required:**

- Permissions: `check_permissions` (polled via `usePermissions`), `request_microphone_permission`,
  `prompt_accessibility_permission`, `open_microphone_settings`, `open_accessibility_settings`.
- Model: `list_models`, `download_model`, `cancel_model_download` and the `model-download` event
  (via `useModels`). Consent button and step-4 readiness both read `progress` / `models`.
- Dictation: `start_dictation`, `stop_dictation`, `get_pipeline_state`, `get_last_result`, the
  `pipeline-state` and `transcription-result` events, and `audio-level` for the inline meter
  (via `usePipeline`). The **raw→cleaned diff needs nothing new**: `TranscriptionResult` already
  carries `raw`, `text`, and `refined`.
- Settings: `save_settings` / `get_settings` (via `useSettings`) for `sttModelId`, the
  `onboardingCompleted` flip, and F12's reset.

**The in-window test field needs no new IPC.** It is a normal focused text input; the existing
paste path delivers the synthetic ⌘V into it (§4.4). The only behavioral note for the Rust side:
none — the pipeline does not need to know it is pasting into our own window.

**Settings fields touched:** `sttModelId` (consent / model picker), `onboardingCompleted`
(finish, skip-anyway, F12 reset). No schema change; no new persistence store (00 §8.7). Defaults
already satisfy "fully working with zero config": `base.en`, Standard mode, `llm.provider:none`,
`insertMethod:paste`, `restoreClipboard:true` (`settings.rs` Default).

**New frontend-only work (no contract change):** the five-step `Onboarding.tsx` rewrite — new
copy, the consent/progress button, the disclosure for other models, the in-window test field,
the diff panel, the honest-skip summary screen, and the F12 entry point in `GeneralTab`
(`update({ onboardingCompleted: false })`).

**Explicitly NO telemetry and NO completion metrics (00 §8.1).** Success is observable only by
the user, on screen, in the moment. There is no event, no counter, no "time to first dictation"
measurement, no funnel. The 60 s target is a design constraint we meet by construction, not a
number the app reports anywhere.

## 7. Out of scope

- Any analytics, funnel, or "onboarding completed" telemetry (constraint 00 §8.1).
- Teaching the dictionary, Polish selection, AI profiles, mode templates, per-mode hotkeys, or
  accuracy upgrades during onboarding — handed to `05-discoverability` and the Settings pages
  (03 §2). This flow teaches exactly three things (§4.5).
- A model-picker matrix on first run (multilingual, large models) — folded behind "Choose
  another model" and otherwise deferred to Settings → Models (03 §2).
- Configuring an AI profile during onboarding (zero-config default is No AI; rules cleanup
  already demonstrates value). Adding AI is F5/F6 (03 §5), reachable from `05` nudges.
- Importing/exporting modes or profiles, Keychain key storage, signed-release notices (roadmap,
  00 §9).
- Any change to the HUD show/hide model (00 §8.2) or the threading model (00 §8.4).
- A guided tour overlay on the real Settings window — re-running onboarding (F12) is the only
  recovery surface in v2.
