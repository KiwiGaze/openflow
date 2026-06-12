# 02 — UX audit: first-run, learnability, and feedback

Status: audit document. Written 2026-06-11 against the `ux-v2` worktree, which holds the
**committed pre-Refine baseline** (verified: `settings.rs` is schema v1 with `llm: LlmConfig`
and `provider: none`; the tab is "AI Provider"; there is no Polish hotkey, no
`refineAfterDictation`, no `profiles.rs`). The in-flight Refine redesign (`docs/REFINE.md`) is
**not** in this worktree. Findings that Refine already fixes are tagged
"already addressed in flight" and are not counted as open problems. Ground truth for facts and
vocabulary is `docs/design/00-current-state.md`; constraints are its §8.

## Executive summary

Velata's dictation happy path is genuinely good: hold a key, speak, get clean text, and the
failure policy never drops your words. The UX debt is concentrated in **discoverability and
naming**, not in the core loop. Several of the app's best features — hands-free tap-to-latch,
the empty-instruction Polish fallback, Copy Last Result, the dictation→rules fallback — have
**zero teaching surface**: a first-time user cannot find them, and onboarding never mentions
them. The ModesTab packs two conflicting gestures onto one row. The HUD says "Polishing…" for
three different operations. Onboarding teaches 1 of the app's 5 capabilities and cannot be
re-run. Accessibility of the settings UI is thin: the HotkeyRecorder is a screen-reader trap,
radio rows have no accessible names, and the audio meter is the only "recording" cue for a
sighted-but-not-hearing user.

Counts: **34 findings** — Critical 4, High 11, Medium 13, Low 6. The 10 quick wins at the end
are all S-effort copy/markup changes that close most of the first-run confusion.

## Table of contents by severity

### Critical (blocks first-run success or loses trust)

- UX-01 — Onboarding can be completed with no model installed via "Skip setup" / "Finish", leaving dictation permanently broken with no in-app recovery
- UX-02 — Onboarding teaches only dictation; rewrite hotkey, modes, dictionary, and AI are never introduced and there is no way to re-run onboarding
- UX-03 — HotkeyRecorder is a keyboard/AT trap: it swallows all keystrokes globally with no visible affordance for how to commit or that Esc cancels
- UX-04 — "Copy Last Result" and the in-app result are the only recovery for a mispasted/lost dictation, but nothing in the product teaches that this safety net exists

### High (causes abandonment or sustained confusion)

- UX-05 — ModesTab row carries two conflicting gestures (radio = activate, click = edit) with no visual hint which is which
- UX-06 — "Modes" as a noun does not say it controls writing style; first-timers cannot guess what the tab does
- UX-07 — Hands-free tap-to-latch is hidden in one hint line and contradicts the "Press to start / stop" toggle users will reach for instead
- UX-08 — HUD shows "Polishing…" for the dictation LLM pass, the rules pass, AND can't distinguish dictation-refine from selection-rewrite
- UX-09 — Rewrite selection requires an AI provider, but the only way to discover this is to trigger it and read an error
- UX-10 — Dictionary empty state ("No entries yet.") is a dead end that never says why the feature matters or how an entry helps
- UX-11 — "Restore clipboard" label/hint don't explain the tradeoff; defaulting on can silently clobber a user's clipboard workflow expectations
- UX-12 — No global "cancel recording" affordance; Esc is unbound and the HUD is click-through, so a mistaken recording can only be ended by speaking or quitting
- UX-13 — Toggling the active mode's "Uses AI" off in ModesTab silently changes dictation behavior with no confirmation or cross-reference to the provider
- UX-14 — Settings window has no keyboard-reachable close/escape and steals the Dock icon; closing is mouse-only (red traffic light)
- UX-15 — Model download failure mid-onboarding is invisible: the error is logged and the row silently reverts to a "Download" button

### Medium (friction, learnability)

- UX-16 — "Dictation style" is an opaque label for the hold-vs-toggle behavior switch
- UX-17 — Tray header reads "Mode" (singular, disabled) with no explanation that the radio list switches writing style
- UX-18 — "Literal" mode name is jargon; users won't know it means "raw words, no cleanup"
- UX-19 — Onboarding "Skip setup" is silent — no summary of what was skipped or how to finish later
- UX-20 — Notice/error copy is inconsistently capitalized and mixes sentence styles across surfaces
- UX-21 — "List installed models" for Ollama is buried under the Model field and only appears for Ollama; users type model names blind
- UX-22 — Spoken-language select offers 14 languages but silently does nothing for the default English-only model; the relationship is one easily-missed hint
- UX-23 — The 5-minute recording cap and the auto-stop are undocumented; a long dictation just ends mid-sentence
- UX-24 — "New mode" creates a mode named "New mode" with a generic prompt and no guidance on what to write
- UX-25 — Last result card shows raw transcript only when refined and different, with no label explaining the difference between "raw" and final
- UX-26 — Accessibility step copy frames the permission as paste-only, hiding that Rewrite/selection capture also needs it
- UX-27 — Onboarding model step hard-codes 3 "starter" models; a user who wants multilingual must finish onboarding first, then discover the full list
- UX-28 — No empty/zero state for "no last result yet" — the Last result card and Copy Last Result simply do nothing with no feedback

### Low (polish)

- UX-29 — Audio level meter has no non-visual equivalent; `aria-hidden` bars are the only "I'm listening" signal
- UX-30 — Toggle components fall back to `aria-label="toggle"` when no label is passed; some are unlabeled to AT
- UX-31 — Sidebar tabs are `<button>`s without `role="tab"` / `aria-selected`, so AT users don't hear a tablist
- UX-32 — `formatBytes` shows decimal MB (148.0 MB) while the README table shows 148 MB; minor cross-surface mismatch
- UX-33 — "Test connection" success message ("Connected — model responded.") disappears on any field edit, so users can't keep a green check while adjusting
- UX-34 — HUD notice and error share near-identical durations and only differ by background tint; color is the sole channel

---

## Critical

### UX-01 — Onboarding exit leaves a non-functional app with no in-app recovery

- Severity: Critical
- Dimension: Workflow bottleneck / missing onboarding
- Evidence: `Onboarding.tsx:30-32` `finish` only sets `onboardingCompleted: true`; "Skip setup" (`Onboarding.tsx:199`) and the step-4 "Finish" button (`Onboarding.tsx:224`) both call it unconditionally. Only the step-3 _Continue_ button is gated (`disabled={step === 3 && !selectedModel?.installed}`, `Onboarding.tsx:216`). The pipeline then hard-fails every dictation: `pipeline.rs:228-232` returns `AppError::Model("speech model not downloaded yet — open Settings to install it")`. That error shows in the HUD for 4 s and clears (`pipeline.rs:154-163`).
- Why it hurts: A user who clicks "Skip setup" on step 1 (a normal instinct — "let me look around first") lands in a fully installed app where the headline feature silently does nothing. The HUD error flashes at the bottom of the screen for 4 seconds and vanishes; there is no persistent "you need a model" banner in the settings window, and the settings UI never points back at onboarding.
- Recommended fix: (a) In `GeneralTab` Speech-recognition card, when no model is installed, show a persistent inline callout: "No speech model installed — dictation is disabled. Download one below." (b) Keep Skip allowed, but if the user skips before installing a model, surface that same callout. (c) Add a re-run entry point (see UX-02). Do not block Skip — blocking violates the app's low-friction principle; guide instead.
- Effort: M

### UX-02 — Onboarding teaches one of five capabilities and cannot be re-run

- Severity: Critical
- Dimension: Missing onboarding / hidden functionality
- Evidence: `Onboarding.tsx:7` `STEPS = ['Welcome', 'Microphone', 'Accessibility', 'Speech model', 'Try it']`. The five panes (`Onboarding.tsx:50-196`) cover only dictation. `onboardingCompleted` is set once and never reset anywhere in the codebase (grep: only writes are `Onboarding.tsx:31` and the Rust default `settings.rs:114`; reads are `App.tsx:29`, `main.rs:87`). No tab, About link, or tray item re-opens onboarding.
- Why it hurts: Rewrite selection (`⌥⇧Space`), the Modes system, the personal Dictionary, and AI refinement are all invisible after the welcome flow. A user who finishes onboarding believes Velata is "a dictation box" and never discovers the rewrite hotkey or that misrecognized names can be fixed permanently. There is no second chance: once dismissed, the only re-entry is editing `settings.json` by hand.
- Recommended fix: (a) Add an About-tab button "Replay setup guide" that sets `onboardingCompleted: false`. (b) Add a final onboarding pane (or extend "Try it") that names the other four features in one line each with their hotkey/tab, e.g. "Rewrite selected text — select, hold ⌥⇧Space, say the change", "Teach it a word — Settings → Dictionary". (c) Mention the rewrite hotkey on the Welcome pane next to the dictation hotkey.
- Effort: M

### UX-03 — HotkeyRecorder is a keyboard/assistive-tech trap

- Severity: Critical
- Dimension: Accessibility & input
- Evidence: `HotkeyRecorder.tsx:14-38` — while recording, a capture-phase `keydown` listener calls `ev.preventDefault(); ev.stopPropagation()` on **every** key and only releases on a valid accelerator or `Escape`. The only affordance is the button title attribute (`HotkeyRecorder.tsx:48`, hover-only) and the in-button text "Press shortcut…" (`HotkeyRecorder.tsx:50`). There is no `aria-label`, no `role`, no live-region announcement, and Tab/Enter/Space are all swallowed so a keyboard user cannot move focus away except by clicking elsewhere (blur handler, `HotkeyRecorder.tsx:29-31`).
- Why it hurts: A keyboard-only or screen-reader user who activates the recorder is stuck: Tab is consumed, Escape is the only exit and is never announced. Even sighted mouse users get no on-screen instruction once recording starts — the "Esc to cancel" lives only in a tooltip. This is the single worst accessibility defect in the app.
- Recommended fix: (a) Render a visible helper line while recording: "Press a shortcut, or Esc to cancel." (b) Give the button `aria-label={`${title}, current shortcut ${formatAcceleratorMac(value)}, activate to record`}` and `aria-describedby` the helper. (c) Announce start/result via an `aria-live="assertive"` region. (d) Let Tab out of recording mode (treat Tab as cancel) so focus is never trapped.
- Effort: M

### UX-04 — The dictation safety net (Copy Last Result / last result) is never taught

- Severity: Critical
- Dimension: Hidden functionality / feedback
- Evidence: Recovery exists in two places — the tray item `"Copy Last Result"` (`tray.rs:53-58`, handler `tray.rs:83-92` writes `result.text` to clipboard) and the GeneralTab "Last result" card with a Copy button (`GeneralTab.tsx:179-193`). Neither is mentioned in onboarding, the HUD, or any hint. The clipboard-fallback notice "Copied to clipboard — press ⌘V to paste (grant Accessibility to auto-paste)" (`pipeline.rs:481-484`) is the only place the word "clipboard" surfaces during use, and it auto-clears in 4 s.
- Why it hurts: When a paste lands in the wrong window, or the user clicks away mid-pipeline, the text is _not lost_ — it's the last result and it's recoverable. But because nothing teaches this, the user's lived experience is "my dictation vanished." That is a trust-losing moment for a tool whose core promise is "never lose your words." The safety net exists; the UX hides it.
- Recommended fix: (a) Onboarding "Try it" pane: add a line "Lost a result? The menu-bar icon → Copy Last Result always has your last dictation." (b) When the clipboard-fallback notice fires, the HUD copy is good but transient; reinforce by keeping the Last result card prominent. (c) Consider naming the tray item "Copy last dictation" for clarity (see naming note).
- Effort: S

---

## High

### UX-05 — Two conflicting gestures on one ModesTab row

- Severity: High
- Dimension: Confusion point
- Evidence: `ModesTab.tsx:52-76` — the row `div` has `onClick` that sets `selectedId` (selects for editing). Inside it a `<label>` wraps a radio whose `onChange` saves `activeModeId` (activates), and the label stops propagation (`ModesTab.tsx:60-64`) so clicking the radio does not also select-for-edit. The only visual feedback is `mode-selected` background (`styles.css:416-418`) for the edit target and the radio dot for active — two independent states on one row with no label distinguishing them.
- Why it hurts: A user who wants to _use_ the Email mode clicks the row text and sees the editor change but the active mode (radio) unchanged — they think they switched but didn't. Conversely, clicking the radio activates without opening the editor. Active vs. selected-for-editing is an expert mental model; first-timers conflate them. 00-current-state.md §1.5 calls this out as the canonical confusion example.
- Recommended fix: Keep the idiom (it's an established pattern, §10) but make the two states legible: (a) label the radio column with a header or `aria-label="Use this mode"`; (b) show an explicit "Active" pill on the active row and "Editing" affordance on the selected row; (c) add one hint under the list: "Click the circle to switch modes. Click a name to edit it."
- Effort: M

### UX-06 — "Modes" doesn't say it controls writing style

- Severity: High
- Dimension: Naming and terminology
- Evidence: Tab label `{ id: 'modes', label: 'Modes' }` (`App.tsx:12`); card header `<h2>Modes</h2>` (`ModesTab.tsx:46`); the only explanation is the sub-hint "The active mode shapes how transcripts are written out." (`ModesTab.tsx:47-50`). The tray header is the bare word "Mode" (`tray.rs:38`).
- Why it hurts: "Mode" is one of the most overloaded words in software (edit mode, dark mode, airplane mode). A user scanning the sidebar cannot predict that "Modes" means "Standard / Email / Notes output styles." The canonical vocabulary (§7) requires keeping the term "Mode," so the fix is a clarifying subtitle, not a rename.
- Recommended fix: Keep the tab label "Modes" (vocabulary constraint) but add a one-line tab subtitle / card subhead that front-loads the meaning: "Output styles — how your dictation is written (Standard, Email, Notes…)." Mirror the same phrasing in the tray header tooltip.
- Effort: S

### UX-07 — Hands-free tap-to-latch is hidden and contradicted by the toggle option

- Severity: High
- Dimension: Hidden functionality / confusion
- Evidence: Tap-to-latch logic lives in `pipeline.rs:193-214` (`on_hotkey_released`: a sub-350 ms hold returns early, keeping recording). The only teaching is one hint: "Hold and speak; release to insert. A quick tap latches hands-free mode." (`GeneralTab.tsx:47-48`) and one onboarding-free line. Meanwhile the "Dictation style" select offers "Press to start / stop" (`GeneralTab.tsx:62-64`, `toggle` behavior), which is a _different_ mechanism. With `HotkeyBehavior::Toggle` the tap-latch path is skipped entirely (`pipeline.rs:199-201`).
- Why it hurts: There are effectively two hands-free models — implicit tap-to-latch (in Hold mode) and explicit toggle mode — and the UI presents neither clearly. A user who wants hands-free will pick "Press to start / stop" from the dropdown (it sounds right), never discovering that a quick tap in the default mode already does it. The word "latch" is jargon.
- Recommended fix: (a) Rewrite the dictation hint in plain words: "Hold to talk; release to insert. Tip: a quick tap keeps recording hands-free until you tap again." (b) Rename the dropdown option "Press to start / stop" → "Tap to start, tap to stop" so the two models read as the same idea. (c) Teach tap-to-latch in onboarding "Try it."
- Effort: S

### UX-08 — "Polishing…" is overloaded across three operations

- Severity: High
- Dimension: Feedback and state visibility
- Evidence: `hudState.ts:11-12` — the `refining` state always returns "Polishing…", regardless of `state.job`. But `refining` is entered for (1) the dictation LLM pass (`pipeline.rs:399`) and (2) selection rewrite (`pipeline.rs:443`), and the same label covers the rules-cleanup-as-fallback case visually. The `transcribing` and `recording` states _are_ job-aware (`hudState.ts:7-9` distinguishes "Listening for instruction…"), making the refining gap inconsistent.
- Why it hurts: When a user rewrites a selection ("make this shorter"), the HUD says "Polishing…" — the same word it shows for ordinary dictation cleanup. They can't tell whether their rewrite instruction was understood or whether it's just doing dictation. For rules-based cleanup (no LLM), "Polishing…" overclaims AI involvement. REFINE.md §"HUD feedback" designs job-aware refining labels ("Rewriting…" / "Polishing selection…") — **partially overlaps with in-flight work**, but the dictation-vs-rewrite distinction in _this_ committed code is unaddressed and worth fixing even before Refine lands.
- Recommended fix: Make `refining` job-aware now: `dictation → "Cleaning up…"`, `refineSelection → "Rewriting…"`. (Refine will extend this with Polish; the two-way split is the minimum.) This is a pure `hudState.ts` change.
- Effort: S

### UX-09 — Rewrite selection's provider requirement is discoverable only by failing

- Severity: High
- Dimension: Confusion / missing explanation
- Evidence: `pipeline.rs:234-239` — starting a `RefineSelection` job with `provider == None` returns `AppError::Llm("rewriting needs an AI provider — configure one in Settings")`. There is no proactive signal: the rewrite hotkey row in GeneralTab (`GeneralTab.tsx:66-71`) says nothing about needing AI, and the README only mentions it in passing (`README.md:85`).
- Why it hurts: A user reads "Rewrite selection — Select text anywhere, hold, and speak an instruction," tries it on a fresh install (no provider), and gets a 4-second error they may not even see. The feature appears broken. The dependency (rewrite ⇒ AI provider) is real and unavoidable, so it should be stated where the hotkey is configured.
- Recommended fix: Add a hint to the Rewrite-selection row: "Needs an AI provider (Settings → AI Provider)." When `provider === 'none'`, optionally show the row in a muted state with "Set up an AI provider to use this."
- Effort: S

### UX-10 — Dictionary empty state is a dead end

- Severity: High
- Dimension: Missing empty state / onboarding
- Evidence: `DictionaryTab.tsx:69-70` — when `settings.dictionary.length === 0`, the entire list area is the single line "No entries yet." The card header is "Personal dictionary" with an explanation (`DictionaryTab.tsx:34-38`), but the empty state itself offers no example, no motivation, no link to where misrecognitions happen.
- Why it hurts: The dictionary is one of the highest-leverage features (permanently fix a misheard name), but a user only finds the tab by browsing, and the empty state gives them nothing to act on. "No entries yet." is a full stop, not an invitation.
- Recommended fix: Replace the empty state with a worked example and motivation: "Nothing here yet. When the transcriber mishears a name or term, add it: type what it heard on the left, the correct spelling on the right. Example: 'open flow' → 'Velata'." The form placeholders already show this (`DictionaryTab.tsx:48,57`) — echo them in the empty state so the connection is explicit.
- Effort: S

### UX-11 — "Restore clipboard" hides its tradeoff

- Severity: High
- Dimension: Naming / excessive configuration
- Evidence: `GeneralTab.tsx:163-169` — Row title "Restore clipboard", hint "Put your previous clipboard back after pasting." Default is `true` (`settings.rs:112`). Behavior: `output.rs:182-187` saves the prior clipboard and restores it after the paste settle delay.
- Why it hurts: The label states _what_ the toggle does mechanically but not _why a user would turn it off_, and the default-on behavior has a subtle failure mode: the restore happens after `KEYSTROKE_SETTLE` (140 ms, `output.rs:36`), so a user who immediately ⌘V again expects the dictated text but gets their old clipboard back. There's no hint about this timing or the reason the option exists (some apps paste asynchronously). Most users never need to see this at top level.
- Recommended fix: (a) Clarify the hint: "After pasting, put whatever you had copied before back on the clipboard. Turn off if you want the dictated text to stay copied." (b) Consider moving this and "Insert method" detail into an "Advanced" disclosure so the Output card defaults to one visible control.
- Effort: S

### UX-12 — No way to cancel a recording you started by mistake

- Severity: High
- Dimension: Hidden functionality / feedback
- Evidence: `cancel()` exists (`pipeline.rs:320-325`) and is exposed as the `cancel_dictation` command (`commands.rs:123-126`), but nothing user-facing calls it. The HUD is click-through (`hud.rs:22` `set_ignore_cursor_events(true)`), so it can't host a cancel button. Esc is not registered as a hotkey (only dictation + rewrite are, `shortcuts.rs:26-45`). 00-current-state.md §5 confirms "Esc is not bound."
- Why it hurts: User taps the hotkey (latching hands-free), then realizes they don't want to dictate. The only ways to stop are: tap again (which _processes_ the audio and pastes whatever noise was captured), or quit from the tray. There is no "never mind." Worst case, an accidental latch in Hold mode keeps recording until the 5-minute cap.
- Recommended fix: Bind Esc to `cancel()` while the pipeline is recording (a transient global shortcut registered on `start`, unregistered on `finish`/`cancel`). Teach it in the HUD label during recording: append a faint "Esc to cancel" or document it in onboarding. (Esc-while-recording does not conflict with the HotkeyRecorder, which is window-scoped.)
- Effort: M

### UX-13 — Turning off a built-in's AI in ModesTab is invisible and unguarded

- Severity: High
- Dimension: Confusion / feedback
- Evidence: For custom modes, "Uses AI" is a toggle (`ModesTab.tsx:105-113`) that flips `usesLlm` immediately via `patchMode`. There is no confirmation and no cross-reference to whether a provider is even configured. The downstream effect (`pipeline.rs:398`) is that `usesLlm && llm_available` gates the entire LLM path — flipping it off silently converts that mode to rules-only cleanup.
- Why it hurts: A user editing a custom mode toggles "Uses AI" while experimenting, forgets, and later wonders why that mode produces blunter output than Standard. The toggle changes a behavioral contract with no acknowledgement and no hint that "off = fast rules cleanup only." Combined with UX-09 (provider state is invisible here), users can't reason about why AI did or didn't run.
- Recommended fix: Add a state-aware hint under the toggle: when on and no provider configured, "No AI provider set up — this mode falls back to rules cleanup. Configure one in AI Provider." When off, "Rules-based cleanup only; nothing is sent to AI." This ties the mode's behavior to the provider tab.
- Effort: S

### UX-14 — Settings window is closeable only with the mouse

- Severity: High
- Dimension: Accessibility & input
- Evidence: `main.rs:92-104` — the only close path is the window's red traffic-light (`CloseRequested`), which hides the window and drops back to Accessory policy. There is no Cmd+W handler, no in-window "Done"/close button, and no Escape-to-close. The sidebar (`App.tsx:35-48`) and content have no keyboard route to dismiss the window.
- Why it hurts: A keyboard-only user who opens Settings from the tray cannot close it without reaching for the mouse to hit the traffic-light (which itself is a small target). On macOS, Cmd+W is the universal expectation for closing a window and it does nothing here.
- Recommended fix: Register Cmd+W (and optionally Escape) to trigger the same hide path as the close button. This is a small Tauri menu/accelerator addition; it does not affect the HUD invariant.
- Effort: S

### UX-15 — Model download failure during onboarding is silent

- Severity: High
- Dimension: Feedback / error states
- Evidence: In onboarding, `download(model.id)` is called (`Onboarding.tsx:160-162`) → `hooks.ts:108-115` invokes `downloadModel` and refreshes. On failure, the Rust side logs and emits a `DownloadProgress` with `done: true, error: Some(...)` (`models.rs:219-228`), but the onboarding UI only reads `p.done` to flip back to the Download button (`Onboarding.tsx:151-166`) and never renders `p.error`. The error field exists in the type (`types.ts:106`) but no onboarding code path displays it.
- Why it hurts: A user on flaky Wi-Fi taps Download, the spinner/percentage shows, then it silently reverts to "Download" with no explanation. They tap again, same result, and conclude the app is broken — at the exact step that gates Continue (UX-01). A network error here is common and completely unexplained.
- Recommended fix: In the onboarding model row (and GeneralTab), when `progress[model.id]?.error` is set, render it: "Download failed: {error}. Check your connection and retry." Reset the error on retry. This reuses data already on the wire.
- Effort: S

---

## Medium

### UX-16 — "Dictation style" is an opaque label

- Severity: Medium
- Dimension: Naming
- Evidence: `GeneralTab.tsx:55-65` — Row title "Dictation style", hint "How the dictation hotkey behaves.", options "Hold to talk" / "Press to start / stop". The word "style" collides with Modes (which are the actual _output_ styles).
- Why it hurts: "Dictation style" sounds like it picks Email vs Notes (output style), but it actually picks the _gesture_ (hold vs toggle). The collision with Modes ("output style") is a genuine terminology clash within one app.
- Recommended fix: Rename to "Hotkey behavior" or "How to record", e.g. Row title "When I press the hotkey" with options "Hold to talk" / "Tap to start, tap to stop". Reserve "style" for Modes.
- Effort: S

### UX-17 — Tray "Mode" header is bare and disabled

- Severity: Medium
- Dimension: Naming / confusion
- Evidence: `tray.rs:38` — `MenuItem::with_id(app, "header", "Mode", false, ...)` renders a disabled "Mode" label above the radio list. No subtitle, no hint.
- Why it hurts: In the menu bar, a lone disabled "Mode" above a list of names ("Standard / Email / Notes / Literal") doesn't explain that these switch how dictation is written. Tray menus are glanced at, not studied.
- Recommended fix: Change the header to "Output mode" or "Writing style" so the menu reads as "Writing style: Standard / Email / …". Keep it disabled (it's a section header).
- Effort: S

### UX-18 — "Literal" mode name is jargon

- Severity: Medium
- Dimension: Naming
- Evidence: `modes.rs:55-61` — the built-in mode `name: "Literal"`, `uses_llm: false`. Its behavior (raw transcript + dictionary, no cleanup) is documented only in 00-current-state.md, not in the UI; the ModesTab editor shows "Uses AI: No" (`ModesTab.tsx:103-104`) for it but no prose.
- Why it hurts: "Literal" is accurate to a linguist but opaque to a user choosing a mode. They can't tell it from "Standard" without trying both. There's no description string for built-in modes in the editor.
- Recommended fix: Either rename to "Verbatim" with a one-line description, or — better — add a `description` field to built-in modes and show it in the editor: Literal → "Exactly what you said. No filler removal, no AI — just your words plus dictionary fixes." (Adding a description field crosses IPC; coordinate per §8.3.)
- Effort: M

### UX-19 — "Skip setup" is silent

- Severity: Medium
- Dimension: Missing onboarding
- Evidence: `Onboarding.tsx:198-201` — "Skip setup" calls `finish` with no confirmation, no summary, and no record of what was skipped. 00-current-state.md §4 confirms "Skip is silent."
- Why it hurts: A user who skips on step 2 doesn't know they still need a model (UX-01), nor that mic/Accessibility were never granted. They're dropped into a half-configured app with no breadcrumb.
- Recommended fix: On skip, route the user to a "Setup incomplete" state or at minimum a toast/banner: "Setup skipped. You can finish anytime from About → Replay setup. Dictation needs a speech model." Pairs with UX-02's re-run entry.
- Effort: S

### UX-20 — Notice/error copy is stylistically inconsistent

- Severity: Medium
- Dimension: Naming / feedback
- Evidence: Lowercase-leading messages: "busy — try again in a moment" (`pipeline.rs:222`), "didn't catch anything" (`pipeline.rs:366`), "rewriting needs an AI provider — configure one in Settings" (`pipeline.rs:237`), "select some text first, then hold the rewrite hotkey" (`pipeline.rs:244`). Sentence-case: "Copied to clipboard — press ⌘V to paste…" (`pipeline.rs:482`). Error-prefixed: `AppError` variants prepend "audio error:", "AI provider error:", etc. (`error.rs:7-21`), so HUD errors read "AI provider error: provider timed out after 30s".
- Why it hurts: The HUD shows a grab-bag of styles — some start lowercase, some are full sentences, some carry a developer-ish "x error:" prefix. It reads unpolished and the prefixes leak internal taxonomy to users.
- Recommended fix: (a) Sentence-case all user-facing notices/errors. (b) Drop the "x error:" prefixes from `AppError::Display` for the variants that reach the HUD, or strip them before display in `set_transient`. (c) Adopt one voice: short imperative sentences ("Couldn't catch that — try again.").
- Effort: M

### UX-21 — "List installed models" for Ollama is buried and provider-specific

- Severity: Medium
- Dimension: Workflow / confusion
- Evidence: `ProviderTab.tsx:118-133` — the Model field is a free-text input; only when `provider === 'ollama'` does a quiet "List installed models" button appear below it (`ProviderTab.tsx:127-131`), populating chips (`ProviderTab.tsx:134-154`). For openaiCompatible there is no discovery at all.
- Why it hurts: A user setting up Ollama must know to click the faint quiet-styled button to see what they have; otherwise they type "qwen2.5:3b" from memory and get a runtime error if it's not pulled. The model name is the most error-prone field and the help is least prominent.
- Recommended fix: For Ollama, fetch the model list on field focus (or on provider select) and present it as the primary control (a select populated from `/api/tags`) with free-text as the fallback. This is in the vicinity of REFINE.md's provider redesign — **the tab is being replaced (already addressed in flight for structure)**, but model discovery prominence is not specified there, so flag it for the new RefineTab editor.
- Effort: M

### UX-22 — Spoken-language select silently no-ops for English-only models

- Severity: Medium
- Dimension: Excessive configuration / confusion
- Evidence: `GeneralTab.tsx:138-149` — a 14-language select bound to `settings.language`, hint "English-only models ignore this." The default model is `base.en` (`settings.rs:108`), which is English-only (`models.rs:39-46` `multilingual: false`).
- Why it hurts: A non-English user on the default model sets "Spoken language: Spanish," dictates Spanish, and gets garbage — because `base.en` ignores the setting and the only warning is a 12px hint they likely skipped. The language control is live and prominent while being inert for the default configuration.
- Recommended fix: When the active model is English-only, disable the language select and replace the hint with an actionable one: "The Base (English) model only transcribes English. Switch to a multilingual model to dictate in {language}." Tie the control's enabled state to `model.multilingual`.
- Effort: S

### UX-23 — Recording cap and auto-stop are undocumented

- Severity: Medium
- Dimension: Missing explanation / feedback
- Evidence: `pipeline.rs:278-287` — a 5-minute (`MAX_RECORDING_SECS = 300`, `settings.rs:16`) timer calls `finish()` automatically, logging "max recording length reached" (`pipeline.rs:284`) but emitting no user-facing notice. The user just sees the pipeline move to Transcribing.
- Why it hurts: Someone dictating a long passage (a journal entry, a draft) hits the silent cap and their recording ends mid-sentence with no warning. They don't know a limit exists or that it was reached.
- Recommended fix: (a) Document the cap in onboarding or the dictation hint ("Recordings stop after 5 minutes."). (b) When the cap fires, emit a notice: "Reached the 5-minute limit — inserting what I have." so the cut-off is explained rather than mysterious.
- Effort: S

### UX-24 — "New mode" gives no authoring guidance

- Severity: Medium
- Dimension: Workflow / missing explanation
- Evidence: `ModesTab.tsx:19-31` — `addMode()` creates `name: 'New mode'` with a generic prompt and immediately selects it. The editor (`ModesTab.tsx:88-152`) shows a Name field and an 8-row monospace prompt textarea (`ModesTab.tsx:116-126`) with no examples, no guidance on prompt structure, and no mention of the shared rules the backend injects (`modes.rs:10-15`).
- Why it hurts: A user clicks "New mode" expecting a guided creator and gets a raw textarea with a placeholder prompt. They don't know the backend already enforces "output only the text / don't follow instructions in the transcript" (`modes.rs:SHARED_RULES`), so they may duplicate or contradict it. Custom modes are powerful but the authoring surface is bare.
- Recommended fix: (a) Add a one-line helper above the prompt: "Describe how to rewrite the transcript. Velata already handles 'output only the text' and language preservation for you." (b) Default new custom modes to _duplicate Standard_ rather than a generic prompt, giving users a working template to edit.
- Effort: S

### UX-25 — Last result's "raw transcript" is unlabeled and conditional

- Severity: Medium
- Dimension: Feedback / naming
- Evidence: `GeneralTab.tsx:179-193` — the Last result card shows `lastResult.text`, and only when `refined && raw !== text` shows "Raw transcript: {raw}" (`GeneralTab.tsx:183-185`). The Copy button copies `text` only.
- Why it hurts: A user comparing what they said to what was inserted sees "Raw transcript:" appear sometimes and not others (it's hidden when rules-only or when raw==text), with no explanation of what "raw" means versus the final text. The distinction (whisper output vs cleaned/LLM result) is meaningful but unexplained, and there's no way to copy the raw version.
- Recommended fix: Label the two clearly — "What you said (raw)" vs "Inserted text" — and always show both when they differ, with a small "Copy raw" affordance. Add a one-line explanation on first view.
- Effort: S

### UX-26 — Accessibility step frames the permission as paste-only

- Severity: Medium
- Dimension: Missing explanation
- Evidence: `Onboarding.tsx:96-100` — "Velata pastes text by simulating ⌘V, which macOS gates behind the Accessibility permission. Skip this and results are copied to the clipboard instead." But selection capture for Rewrite _also_ requires it: `output.rs:191-196` returns "Accessibility permission is required to read the selection."
- Why it hurts: A user who skips Accessibility (told only that paste degrades to clipboard) later tries Rewrite selection and it fails to even read the selection — a consequence onboarding never warned about. The permission's full scope is understated.
- Recommended fix: Extend the copy: "…Accessibility also lets Velata read your selected text for the Rewrite feature. Skip and you'll get clipboard-only paste, and Rewrite won't work."
- Effort: S

### UX-27 — Onboarding starter models exclude multilingual; full list is post-onboarding

- Severity: Medium
- Dimension: Workflow / excessive friction
- Evidence: `Onboarding.tsx:10` `STARTER_MODELS = ['base.en', 'small.en', 'large-v3-turbo-q5_0']` — two English-only and one multilingual, filtered at `Onboarding.tsx:25-28`. The registry has 7 models including `base` and `small` multilingual (`models.rs:47-70`), but they only appear in GeneralTab after onboarding completes.
- Why it hurts: A Spanish or Chinese speaker going through onboarding sees mostly English models and one big turbo model; the obvious "Base (Multilingual)" option is hidden. They either download the wrong model or finish onboarding confused, then must find the full list in Settings. The default (`base.en`) is also English-only, compounding UX-22.
- Recommended fix: (a) Add a "Show all models" expander on the onboarding model step, or (b) detect a non-English system locale and surface a multilingual starter. At minimum, include `base` (multilingual) in the starter set so every user has a non-English option in onboarding.
- Effort: M

### UX-28 — No feedback when there is no last result

- Severity: Medium
- Dimension: Empty state / feedback
- Evidence: GeneralTab's Last result card only renders when `lastResult` is truthy (`GeneralTab.tsx:179`). The tray "Copy Last Result" handler does nothing when `last_result()` is `None` (`tray.rs:84-92` — the `if let Some` simply falls through silently).
- Why it hurts: A user who clicks "Copy Last Result" before ever dictating gets no clipboard change and no feedback — it appears broken. Similarly, the absence of the Last result card gives no hint that one will appear after dictating.
- Recommended fix: (a) Tray: when there's no result, either disable the item or, on click with none, flash a HUD notice "No dictation yet." (b) Optionally show a muted placeholder card "Your last dictation will appear here."
- Effort: S

---

## Low

### UX-29 — Audio meter is the only "listening" cue and is `aria-hidden`

- Severity: Low
- Dimension: Accessibility / feedback
- Evidence: `Hud.tsx:33-39` — the level bars are wrapped in `aria-hidden` and are the primary recording signal; the label says only "Listening…" (`hudState.ts:7-8`). A user who can see but is in a noisy/muted environment relies on the bars to confirm the mic hears them.
- Why it hurts: For screen-reader users the HUD is largely invisible anyway (it's a click-through overlay), but more practically, there is no "input detected" confirmation other than the visual meter. If the mic is muted at the OS level, the bars stay flat and nothing says "I hear silence."
- Recommended fix: Low priority given the HUD's overlay nature, but consider a "no input detected" label after a few seconds of flat RMS during recording: "Listening… (no sound yet)".
- Effort: S

### UX-30 — Toggles default to `aria-label="toggle"`

- Severity: Low
- Dimension: Accessibility
- Evidence: `Toggle.tsx:16` — `aria-label={label ?? 'toggle'}`. Callers in GeneralTab pass labels ("Restore clipboard", "Launch at login", `GeneralTab.tsx:167,173`) and ModesTab passes "Uses AI" (`ModesTab.tsx:111`), so most are fine — but the fallback "toggle" would leave any future unlabeled toggle anonymous to AT.
- Why it hurts: Minor today (all current call sites label their toggles), but the fallback is a latent a11y bug. A screen reader on an unlabeled toggle announces just "toggle, switch."
- Recommended fix: Make `label` required in the `Toggle` props (TypeScript-enforced) so every toggle has an accessible name; drop the `'toggle'` fallback.
- Effort: S

### UX-31 — Sidebar tabs aren't a semantic tablist

- Severity: Low
- Dimension: Accessibility
- Evidence: `App.tsx:37-47` — tabs are `<button class="sidebar-item">` with no `role="tab"`, no `aria-selected`, no parent `role="tablist"`, and the content panes have no `role="tabpanel"`.
- Why it hurts: Screen-reader users hear a list of buttons, not "tab 2 of 5, selected," losing the orientation a tablist provides. Keyboard arrow-key navigation between tabs (the ARIA tab pattern) isn't wired either.
- Recommended fix: Add `role="tablist"` to the nav, `role="tab"` + `aria-selected` to each button, and `role="tabpanel"` to `.content`. Optionally support Left/Right arrow navigation.
- Effort: S

### UX-32 — Byte formatting mismatches the README

- Severity: Low
- Dimension: Naming / consistency
- Evidence: `format.ts:13` returns `value.toFixed(1)` for values ≥ 1 and < 100, so `148_000_000` renders "148.0 MB" in `GeneralTab.tsx:96-97` / `Onboarding.tsx:144`, while the README table shows "148 MB" (`README.md:72-74`).
- Why it hurts: Trivial, but a 148.0 MB vs 148 MB mismatch between in-app and docs is the kind of inconsistency that erodes the impression of polish. (For values exactly at boundaries the decimal also looks odd.)
- Recommended fix: Drop the trailing `.0` for whole-hundred MB values, or round model sizes to whole MB in the UI to match the docs.
- Effort: S

### UX-33 — Test-connection result vanishes on any edit

- Severity: Low
- Dimension: Feedback
- Evidence: `ProviderTab.tsx:27-30,32-37` — `patch` and `switchProvider` both call `setTestResult(null)`, so the green "Connected — {model} responded." (`llm.rs:191-194`) disappears the moment the user touches any field.
- Why it hurts: A user who tests successfully, then tweaks the timeout, loses the green confirmation and may think the connection broke. Minor, but the success state is fragile.
- Recommended fix: Only clear the test result when fields that affect connectivity (base URL, key, model, provider) change — not timeout. Or show the last result as stale (greyed) rather than removing it.
- Effort: S

### UX-34 — HUD notice vs error distinguished only by color

- Severity: Low
- Dimension: Accessibility / feedback
- Evidence: `styles.css:612-618` — `.hud-error` (red `rgb(120 24 24)`) and `.hud-notice` (amber `rgb(112 84 16)`) differ only by background; both share the same duration (4 s, `pipeline.rs:37`) and layout. The label text is the only other differentiator and it carries no severity marker.
- Why it hurts: A colorblind user can't tell a recoverable notice ("Copied to clipboard…") from a hard error ("provider timed out") since both are dark pills with white text. Color is the sole severity channel.
- Recommended fix: Add a small leading glyph or word — e.g. a check/ℹ for notices and a ⚠ for errors — so severity survives without color. (The HUD content can be freely restyled per §8.2.)
- Effort: S

---

## Top 10 quick wins (all S effort)

1. **UX-04** — Add one onboarding line teaching "Copy Last Result" as the lost-dictation safety net.
2. **UX-08** — Make the HUD `refining` label job-aware now: "Cleaning up…" (dictation) vs "Rewriting…" (selection). Pure `hudState.ts` change.
3. **UX-09** — Add "Needs an AI provider (Settings → AI Provider)" to the Rewrite-selection hotkey row.
4. **UX-10** — Replace "No entries yet." with a worked example: "'open flow' → 'Velata'".
5. **UX-06** — Add a Modes subtitle: "Output styles — how your dictation is written."
6. **UX-15** — Render `progress[model.id].error` in onboarding/GeneralTab on download failure.
7. **UX-22** — Disable the language select for English-only models with an actionable hint.
8. **UX-16** — Rename "Dictation style" → "Hotkey behavior"; option "Tap to start, tap to stop".
9. **UX-13** — Add a state-aware hint under ModesTab "Uses AI" explaining the rules-fallback.
10. **UX-03 (partial)** — Render a visible "Press a shortcut, or Esc to cancel." line while the HotkeyRecorder is recording (the full a11y fix is M; the helper line is S and removes the worst of it).

## Deliberately NOT raised (would violate §8 constraints or REFINE.md decisions)

- **Show/hide the HUD window for cleaner feedback** — forbidden by §8.2 / the HUD invariant (Tauri #14102). The fade-content approach is mandatory; restyle content only.
- **Persist dictation history / a "recent results" list** to fix the lost-dictation problem more durably — violates the no-transcript-persistence privacy invariant (§8.1, §6). Copy Last Result (in-memory) is the sanctioned ceiling; UX-04 works within it.
- **Auto-configure a default cloud AI provider so refinement "just works"** — violates §8.1 (no default-on network, cloud is opt-in BYO-key). Rules-based cleanup as the zero-config default (§10) is correct; UX-09/UX-13 only make the _existing_ opt-in legible.
- **Replacing `provider: none` with a "No AI" radio, renaming the tab to "Refine", adding a dictation-refine kill switch, and a Polish hotkey row** — these are **already addressed in flight** in REFINE.md (§"Settings UI", §"Tray", §"settings.json v2"). The committed code still shows the old ProviderTab; I did not file the structural provider-tab/profile findings as open, since Refine lands them.
- **Job-aware "Polishing selection…" / "Rewriting…" full label matrix including Polish** — the Polish half is **already addressed in flight** (REFINE.md §"HUD feedback"). UX-08 only raises the dictation-vs-rewrite split that exists in _this_ committed `hudState.ts`.
- **Telemetry-based onboarding funnel analytics** to find where users drop — forbidden by §8.1 (no telemetry). Onboarding fixes here are heuristic, not data-driven.
- **Accounts / cloud sync of settings or dictionary** — forbidden by §8.8 (no accounts, no cloud sync). Dictionary import/export stays the local-file story.
