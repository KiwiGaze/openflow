# 09 — UX polish: tokens, states, copy, and component specs

Status: design proposal. Written 2026-06-11 against the `ux-v2` worktree (committed pre-Refine
baseline — see 02 §intro). Ground truth and vocabulary: `00-current-state.md` (§7 terms, §8
constraints). Structure this applies to: `03-information-architecture.md` (the seven-page v2
sitemap). This file owns the **solutions layer** — exact CSS-variable tokens, per-state specs,
exact replacement copy, component contracts. It does not re-file audit findings; it cites them
by ID (`UX-xx`) from `02-ux-audit.md` and implements their fixes. Discoverability copy and
empty-state catalog live in `05-discoverability.md`; onboarding screens in `04-onboarding.md` —
this doc covers their _visual_ and _token_ layer, not their content.

What is already good and is kept: the design already ships a real dark-mode palette
(`styles.css:18-30`), a real `:focus-visible` ring (`styles.css:70-76`), a `role="switch"`
Toggle (`Toggle.tsx:11-16`), `aria-hidden` on decorative HUD bars (`Hud.tsx:34,40`), and a
disciplined one-row-per-feature card system (`Row.tsx`, `.card`). The polish below tightens
these; it does not rebuild them.

---

## 1. Design-token foundation

### 1.1 What `styles.css` declares today (audit)

| Group         | Declared today                                           | Value(s) quoted                                                                                                                                                                                                                        | Verdict                                                                                                                                  |
| ------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Color (light) | `:root` `styles.css:6-15`                                | `--bg:#f5f5f7`, `--panel:#fff`, `--text:#1d1d1f`, `--text-dim:#6e6e73`, `--border:#d2d2d7`, `--accent:#5b5bd6`, `--accent-text:#fff`, `--danger:#d62b2b`, `--ok:#1f8a4c`, `--chip:#e8e8ed`                                             | Good base; missing semantic roles (no `--accent-hover`, `--bg-elevated`, `--text-on-hud`, warn).                                         |
| Color (dark)  | `@media (prefers-color-scheme: dark)` `styles.css:18-30` | overrides 10 vars: `--bg:#1e1e20`, `--panel:#2a2a2d`, `--text:#f5f5f7`, `--text-dim:#98989d`, `--border:#3d3d41`, `--accent:#7c7cf0`, `--danger:#ff6961`, `--ok:#5dd28a`, `--chip:#3a3a3e`                                             | **Dark mode exists and is real.** But it is `prefers-color-scheme`-only — no user override, no Appearance setting (03 §2 calls for one). |
| Type sizes    | scattered literals                                       | root `13px` (`:1`), `.card h2` `13px` uppercase (`:155-156`), `.row-hint`/`.form-error`/`.privacy-note` `12px`, `.badge`/`.step-dot` `11px`, `.mono` `11px`, `.sidebar-brand` `15px`, `.onboarding-pane h1` `22px`, `.hud-pill` `13px` | No scale; six hard-coded sizes. Needs named steps.                                                                                       |
| Weights       | literals                                                 | `.row-title` `500` (`:177`), `.sidebar-brand`/`.hotkey-chip` `700`/`600`, `.dict-to` `600`                                                                                                                                             | Ad hoc; fold into the type scale.                                                                                                        |
| Line-height   | almost none                                              | `.privacy-list` `1.6` (`:506`), `.onboarding-pane p` `1.5` (`:564`)                                                                                                                                                                    | Body text has no default leading — set one.                                                                                              |
| Radii         | literals                                                 | `6px` (inputs/btn/chip/hotkey), `8px` (`.error-banner`), `10px` (`.card`), `99px` (pills/toggle/badge)                                                                                                                                 | Three real steps + pill; name them.                                                                                                      |
| Spacing       | literals everywhere                                      | `2/4/6/8/10/12/14/16/18/22/28px` across padding/gap                                                                                                                                                                                    | No scale; pick a 4px-based ramp.                                                                                                         |
| Elevation     | one shadow                                               | `.toggle-knob` `box-shadow:0 1px 2px rgb(0 0 0/30%)` (`:325`)                                                                                                                                                                          | No card/sheet/HUD elevation tokens.                                                                                                      |
| Motion        | literals                                                 | `0.15s` toggle, `0.18s` HUD fade, `0.8s` spin, `1.2s` pulse, `80ms` bars                                                                                                                                                               | Fine values; no reduced-motion guard (see §4.6).                                                                                         |

Verdict: the palette is solid and dark mode is genuine; the gap is **named scales** (type,
space, radius, elevation) and **semantic color roles** so HUD/warn/hover stop being magic
numbers. The block below keeps every existing value as the anchor of its scale — nothing
visible shifts on light mode except the additions.

### 1.2 Consolidated `:root` token block (drop-in replacement for `styles.css:1-30`)

```css
:root {
  font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif;
  color-scheme: light dark;

  /* ---- Color: surfaces ---- */
  --bg: #f5f5f7;
  --bg-elevated: #ffffff; /* result-text / inset wells, was bare var(--bg) */
  --panel: #ffffff;
  --border: #d2d2d7;
  --border-strong: #b8b8c0; /* focus targets, active model row outline */

  /* ---- Color: text ---- */
  --text: #1d1d1f;
  --text-dim: #6e6e73;
  --text-faint: #8e8e93; /* timestamps, disabled hint text */

  /* ---- Color: accent + state ---- */
  --accent: #5b5bd6;
  --accent-hover: #4a4ac4; /* btn-primary:hover, was filter(brightness) */
  --accent-text: #ffffff;
  --danger: #c4241f; /* darkened from #d62b2b for margin + dark legibility (§4.5) */
  --danger-bg: #fbeaea;
  --ok: #1f8a4c;
  --warn: #8a5a00; /* notices; AA on --panel; replaces ad-hoc amber */
  --warn-bg: #fdf3e3;
  --chip: #e8e8ed;

  /* ---- Color: HUD (fixed dark pill, both themes) ---- */
  --hud-bg: rgb(28 28 32 / 92%);
  --hud-bg-notice: rgb(122 90 0 / 95%);
  --hud-bg-error: rgb(150 30 28 / 96%);
  --hud-text: #f5f5f7;
  --hud-bars: #9e9efc;

  /* ---- Type scale (size / weight / line-height) ---- */
  --font-size-root: 13px;
  --text-h1: 22px; /* onboarding titles */
  --text-h2: 13px; /* card headers (uppercase) */
  --text-title: 13px; /* row titles, sidebar items */
  --text-body: 13px;
  --text-hint: 12px; /* hints, privacy notes, form errors */
  --text-badge: 11px; /* badges, step dots, mono */
  --weight-regular: 400;
  --weight-medium: 500; /* row titles */
  --weight-semibold: 600; /* hotkey chip, dict-to */
  --weight-bold: 700; /* sidebar brand */
  --leading-body: 1.5;
  --leading-tight: 1.3;
  --tracking-h2: 0.4px; /* uppercase card headers */

  /* ---- Spacing scale (4px base) ---- */
  --space-1: 2px;
  --space-2: 4px;
  --space-3: 6px;
  --space-4: 8px;
  --space-5: 10px;
  --space-6: 12px;
  --space-7: 14px;
  --space-8: 16px;
  --space-9: 18px;
  --space-10: 22px;
  --space-12: 28px;

  /* ---- Radius scale ---- */
  --radius-sm: 6px; /* inputs, buttons, chips, hotkey */
  --radius-md: 8px; /* banners, sheets */
  --radius-lg: 10px; /* cards */
  --radius-pill: 99px;

  /* ---- Elevation ---- */
  --shadow-knob: 0 1px 2px rgb(0 0 0 / 30%);
  --shadow-card: 0 1px 2px rgb(0 0 0 / 4%);
  --shadow-sheet: 0 8px 28px rgb(0 0 0 / 18%); /* template gallery / future sheets */
  --shadow-hud: 0 6px 20px rgb(0 0 0 / 35%);

  /* ---- Motion ---- */
  --motion-fast: 0.15s;
  --motion-hud: 0.18s;
  --ease: ease;

  font-size: var(--font-size-root);
}

@media (prefers-color-scheme: dark) {
  :root[data-theme='system'],
  :root:not([data-theme]) {
    --bg: #1e1e20;
    --bg-elevated: #242427;
    --panel: #2a2a2d;
    --border: #3d3d41;
    --border-strong: #54545a;
    --text: #f5f5f7;
    --text-dim: #98989d;
    --text-faint: #818188;
    --accent: #7c7cf0;
    --accent-hover: #9090f4;
    --danger: #ff6961;
    --danger-bg: #3a1e1d;
    --ok: #5dd28a;
    --warn: #f0b65a;
    --warn-bg: #3a2e16;
    --chip: #3a3a3e;
    /* HUD pill stays its fixed dark palette in both themes. */
  }
}

/* Manual override from General → Appearance (wins over the media query). */
:root[data-theme='light'] {
  color-scheme: light;
  /* inherits the light :root block above — no overrides needed */
}

:root[data-theme='dark'] {
  color-scheme: dark;
  --bg: #1e1e20;
  --bg-elevated: #242427;
  --panel: #2a2a2d;
  --border: #3d3d41;
  --border-strong: #54545a;
  --text: #f5f5f7;
  --text-dim: #98989d;
  --text-faint: #818188;
  --accent: #7c7cf0;
  --accent-hover: #9090f4;
  --danger: #ff6961;
  --danger-bg: #3a1e1d;
  --ok: #5dd28a;
  --warn: #f0b65a;
  --warn-bg: #3a2e16;
  --chip: #3a3a3e;
}
```

Note: the dark values appear twice — the `[data-theme='dark']` selector (explicit "Dark") and
the media-query block ("System" while the OS is dark). CSS cannot share custom-property values
across unrelated selectors without a preprocessor, so keep the two lists identical when either
changes.

### 1.3 Dark mode: verdict and the Appearance setting

**Dark mode already exists today** — `styles.css:18-30` is a complete `prefers-color-scheme:
dark` palette, and every component reads `var(--…)`, so the whole settings window and HUD
already flip with the OS. The two gaps:

1. **No manual override.** 03 §2 (sitemap) and the General page spec call for **Appearance:
   System / Light / Dark**. A user who keeps macOS light but wants OpenFlow dark has no path.
2. **Webview honoring.** Both webviews (`index.html`, `hud.html`) load the same `styles.css`,
   so a setting must reach both. The HUD is already a fixed dark pill, so Appearance only needs
   to drive the _settings_ window in practice — but the mechanism should be uniform.

Delivery plan (CSS-only + one settings field, IPC-mirrored per 00 §8.3):

- **Field.** Add `appearance: "system" | "light" | "dark"` to `Settings` (Rust serde +
  `packages/core/src/types.ts`, default `"system"`). It joins the General page as one Row
  (03 §2 General → Appearance).
- **Apply.** On settings load and on change, the settings webview sets
  `document.documentElement.dataset.theme = settings.appearance`. The CSS above already routes:
  `data-theme='light'` forces light, `data-theme='dark'` forces dark, `data-theme='system'`
  (or absent) defers to the media query. `color-scheme` is set per-branch so native form
  controls and scrollbars match.
- **HUD.** The HUD webview also sets `data-theme`, but since `--hud-*` tokens are theme-fixed it
  is visually a no-op there; setting it keeps the two webviews consistent and future-proofs any
  themed HUD chrome. No new IPC event — the HUD already subscribes to `settings-changed`.
- **No flash.** Read `appearance` from the initial settings payload before first paint (the app
  already gates render on `api` in `App.tsx:25-27`); set `dataset.theme` in that same guard.

Row copy (General page):

> **Appearance** — `System ▾` (System / Light / Dark)
> hint: _"Match macOS, or force light or dark for OpenFlow's windows."_

---

## 2. Terminology consistency sweep

One table. Every user-facing string that should change, quoted from code with `file:line`,
the problem in one phrase, and the replacement. Replacements respect the canonical vocabulary
(00 §7: keep "Mode" = output style; "AI profile" = LLM connection; "Insert"; "Dictionary") and
are plain English. Cross-refs to the audit finding that raised the underlying issue.

| Surface           | Current string (file:line)                                                                                 | Problem                                               | Replacement                                                                                                                          | Ref          |
| ----------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------ |
| Sidebar tab       | `'General'` `App.tsx:11`                                                                                   | grab-bag; 03 splits it                                | becomes 4 pages: `Dictation`, `Output`, `Models`, `General`                                                                          | 03 §2        |
| Sidebar tab       | `'AI Provider'` `App.tsx:14`                                                                               | names the engine, not the job; re-homed               | folds into `Models` page → "AI profiles" card                                                                                        | 03 §3, UX-06 |
| Sidebar / tray    | tab `'Modes'` `App.tsx:12`; header `"Mode"` `tray.rs:38`                                                   | "mode" is overloaded; no hint it = writing style      | keep label `Modes`; add card subhead "Output styles — how your dictation is written"                                                 | UX-06, UX-17 |
| Tray header       | `"Mode"` `tray.rs:38`                                                                                      | bare disabled word                                    | `"Writing style"` (still disabled section header)                                                                                    | UX-17        |
| Tray item         | `"Copy Last Result"` `tray.rs:55`                                                                          | title-case, vague                                     | `"Copy last dictation"`                                                                                                              | UX-04        |
| General → Hotkeys | title `"Dictation style"` `GeneralTab.tsx:55`                                                              | collides with Modes (the real "style"); means gesture | `"When I press the hotkey"`                                                                                                          | UX-16        |
| General → Hotkeys | option `"Press to start / stop"` `GeneralTab.tsx:63`                                                       | reads as a different mechanism than tap-to-latch      | `"Tap to start, tap to stop"`                                                                                                        | UX-07, UX-16 |
| General → Hotkeys | hint `"How the dictation hotkey behaves."` `GeneralTab.tsx:55`                                             | says nothing                                          | _"Hold to talk, or tap once to start and again to stop."_                                                                            | UX-16        |
| Dictation hint    | `"Hold and speak; release to insert. A quick tap latches hands-free mode."` `GeneralTab.tsx:48`            | "latches" is jargon                                   | _"Hold to talk; release to insert. Tip: a quick tap keeps recording hands-free until you tap again."_                                | UX-07        |
| Hotkeys row       | title `"Rewrite selection"` `GeneralTab.tsx:66`                                                            | fine, but gives no AI dependency                      | keep title; add hint _"Select text, hold, and say the change. Needs an AI profile."_                                                 | UX-09        |
| Output row        | title `"Insert method"` `GeneralTab.tsx:154`                                                               | ok per vocab ("Insert")                               | keep; move to Output page (03)                                                                                                       | —            |
| Output row        | title `"Restore clipboard"`, hint `"Put your previous clipboard back after pasting."` `GeneralTab.tsx:163` | hides why you'd turn it off                           | hint → _"After pasting, put back whatever you'd copied before. Turn off to keep the dictated text on the clipboard."_                | UX-11        |
| Modes built-in    | `name: "Literal"` `modes.rs`                                                                               | linguist jargon                                       | keep id; show description _"Exactly what you said — no filler removal, no AI. Just your words plus dictionary fixes."_ in editor     | UX-18        |
| Modes editor      | hint `"Send the transcript to your AI provider with this prompt."` `ModesTab.tsx:102`                      | "provider" should be "profile" (00 §7)                | _"Send the transcript to your active AI profile with this prompt."_ + state-aware fallback line (UX-13)                              | UX-13        |
| Dictionary empty  | `"No entries yet."` `DictionaryTab.tsx:70`                                                                 | dead end                                              | worked example ghost row (05 §3.1): _"Nothing here yet. When a name or term gets misheard, add it — e.g. 'open flow' → 'OpenFlow'."_ | UX-10        |
| HUD label         | `'Polishing…'` for all refining `hudState.ts:11`                                                           | overloaded across 3 ops                               | job-aware: dictation → `"Cleaning up…"`, refineSelection → `"Rewriting…"`                                                            | UX-08        |
| HUD fallback      | `'Something went wrong'` `hudState.ts:17`                                                                  | generic                                               | `"Something went wrong — your text is on the clipboard"` (dictation never loses text, 00 §5)                                         | UX-20        |
| Notice (error)    | `"busy — try again in a moment"` `pipeline.rs:222`                                                         | lowercase, dev voice                                  | `"Still finishing the last one — try again in a moment."`                                                                            | UX-20        |
| Notice            | `"speech model not downloaded yet — open Settings to install it"` `pipeline.rs:230`                        | names failure, not the fix path clearly               | `"No speech model yet. Open Models in Settings to download one."`                                                                    | UX-20, UX-01 |
| Notice (error)    | `"rewriting needs an AI provider — configure one in Settings"` `pipeline.rs:237`                           | "provider" vs "profile"                               | `"Rewrite needs an AI profile. Add one in Settings → Models."`                                                                       | UX-09, UX-20 |
| Notice (error)    | `"select some text first, then hold the rewrite hotkey"` `pipeline.rs:244`                                 | lowercase                                             | `"Select some text first, then hold the rewrite hotkey."`                                                                            | UX-20        |
| Notice            | `"didn't catch anything"` `pipeline.rs:366`                                                                | lowercase, blames nothing                             | `"Didn't catch that — try again."`                                                                                                   | UX-20        |
| Notice            | `"AI cleanup failed ({err}) — inserted plain transcript"` `pipeline.rs:411`                                | leaks raw err; ok intent                              | `"AI cleanup unavailable — inserted the plain transcript."` (log `{err}`, don't show it)                                             | UX-20        |
| Notice            | `"Copied to clipboard — press ⌘V to paste (grant Accessibility to auto-paste)"` `pipeline.rs:482`          | good already — keep                                   | keep verbatim (names the fix; sentence case)                                                                                         | credit       |
| AppError prefixes | `"audio error:"`, `"AI provider error:"`, `"transcription error:"`, `"output error:"` `error.rs:7-21`      | leaks internal taxonomy to HUD                        | strip the `"x error:"` prefix before HUD display (keep in logs); HUD shows the bare message                                          | UX-20        |
| Onboarding        | step name `'Try it'` `Onboarding.tsx:7`                                                                    | fine                                                  | keep; 04 adds a 5th "You're set" success step                                                                                        | 04 §4.5      |
| Onboarding        | Accessibility copy frames as paste-only `Onboarding.tsx:96-100`                                            | hides Rewrite needs it too                            | add _"…it also lets OpenFlow read selected text for Rewrite. Skip and you'll get clipboard-only paste, and Rewrite won't work."_     | UX-26        |

Note on the `provider` → `profile` swap: the Rust strings above say "provider" because the
committed code predates Refine. Once profiles land (REFINE.md), these user-facing strings must
say "AI profile" to match 00 §7. The replacements assume the Refine vocabulary; if a string
ships before Refine, use "AI provider" and rename in the Refine PR.

---

## 3. State coverage matrix

For each v2 surface: does each state exist today (cite), and the spec for the missing ones.
"—" = not applicable to that surface. Surfaces follow 03 §2 pages.

| Surface                  | Loading                                               | Empty                                             | Success                                                                         | Error                                                                                   | Disabled                                                                |
| ------------------------ | ----------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| **Dictation** page       | —                                                     | —                                                 | save is silent (ok)                                                             | save error → top banner `App.tsx:50-57` (exists, see below)                             | language select inert for EN-only models (UX-22): **spec** disable it   |
| **Modes** page           | —                                                     | n/a (always ≥4 built-ins)                         | auto-save silent                                                                | —                                                                                       | "Uses AI" toggle lacks fallback hint (UX-13): **spec** state-aware hint |
| **Models** → Speech      | per-row `%` `GeneralTab.tsx:110-111` (exists)         | n/a (registry static)                             | `installed` badge `:107` (exists)                                               | **MISSING** — download fail silently reverts to Download (UX-15): **spec** below        | radio disabled until installed `:88` (exists, good)                     |
| **Models** → AI profiles | "Testing…" `ProviderTab.tsx:173` (exists)             | REFINE.md empty state (in flight)                 | "Connected — {model} responded." `:176` (exists; fragile, UX-33)                | `form-error` on test fail `:176` (exists)                                               | Test btn disabled on invalid URL `:170` (exists, good)                  |
| **Output** page          | —                                                     | Last result hidden when none `GeneralTab.tsx:179` | result card shows text (exists)                                                 | —                                                                                       | —                                                                       |
| **Dictionary** page      | —                                                     | `"No entries yet."` `:70` (exists, weak — UX-10)  | entry appends (exists)                                                          | `form-error` on bad entry `:67` (exists)                                                | —                                                                       |
| **General** page         | —                                                     | —                                                 | save silent                                                                     | save banner (shared)                                                                    | —                                                                       |
| **About** page           | `info` null → renders nothing `AboutTab.tsx:19`       | **spec** show skeleton lines                      | static (exists)                                                                 | —                                                                                       | —                                                                       |
| **HUD**                  | spinner `Hud.tsx:40` (exists)                         | fades out when idle (exists)                      | **MISSING** — just disappears (00 §F1, UX target): **spec** success flash below | `hud-error` tint `:612` (exists)                                                        | — (non-interactive)                                                     |
| **Onboarding**           | `'checking…'` perm badge `Onboarding.tsx:73` (exists) | —                                                 | step-4 result card (exists)                                                     | **MISSING** download error (UX-15); perm denied → "Open System Settings" `:84` (exists) | Continue gated on model `:216` (exists)                                 |
| **Tray**                 | —                                                     | Copy-last no-ops with no result (UX-28)           | mode check mark (exists)                                                        | —                                                                                       | header disabled `:38` (exists)                                          |

### 3.1 Settings save error banner — assessment

Exists: `App.tsx:50-57` renders `api.saveError` in `.error-banner` (`styles.css:279-289`) with
a Dismiss button. **Keep it** — placement (top of content, above all tabs) is correct and the
token styling (`danger` border, `color-mix` 12% bg) is good. Two refinements:

- Give it `role="alert"` so VoiceOver announces a save failure (it is currently a silent
  `<div>`).
- Use the new `--danger-bg` token instead of the inline `color-mix(in srgb, var(--danger) 12%,
var(--panel))` so light/dark stay tuned. The `color-mix` is clever but un-named.

### 3.2 Model download failure — inline state (NEW, UX-15)

Today the error rides the wire (`DownloadProgress.error`, `types.ts`) but no UI reads it — the
row silently reverts to "Download" (`GeneralTab.tsx:123-132`, `Onboarding.tsx:156-166`). Spec:

```
┌ Base (English)            multilingual ─────────────────────────┐
│ 148 MB — Fast with decent accuracy.                              │
│ ⚠ Download failed — check your connection.   [ Retry ]           │  ← --warn / --warn-bg
└──────────────────────────────────────────────────────────────────┘
```

- Render when `progress[model.id]?.done && progress[model.id]?.error`.
- Copy: `"Download failed — check your connection."` (the raw `error` goes to the log, not the
  row — same no-dev-strings rule as §2). Show on both Models page and onboarding step 3.
- The "Download" button label becomes **"Retry"** in the error state; clicking clears the error
  and re-invokes `download(model.id)`.
- Styling: a one-line `.row-hint` colored `var(--warn)`; no full banner (per-row scope).

### 3.3 LLM test pending / ok / fail — assessment (UX-33)

Exists and is mostly good: `"Testing…"` on the button (`ProviderTab.tsx:173`), green
`badge badge-ok` on ok, `form-error` on fail (`:176`). One real bug: `patch()` and
`switchProvider()` call `setTestResult(null)` (`:28,33`), so the green check vanishes the
instant the user edits _any_ field, including Timeout. Spec: clear the result only when a
_connectivity_ field changes (base URL, key, model, provider) — not Timeout; or grey it as
stale rather than removing it. Keep the existing three visual states.

### 3.4 HUD success moment (NEW — the headline gap)

Today the pipeline goes `inserting → idle` (`pipeline.rs:307-309`) and the HUD content simply
fades out (`hudVisible` false at idle, `hudState.ts:22-24`). There is no confirmation that the
text landed — F1 (03 §5) and 04 §4.4 both call for a brief success flash. Spec (content-only;
the window is never shown/hidden, 00 §8.2):

- **New status** `inserted` (or reuse `notice` with a success variant). On successful insert,
  emit `inserted` with a short preview of the text, hold **≤ 1.5 s**, then fade to idle. The
  generation counter already guards staleness — a new job pre-empts it cleanly.
- **Visual:** a check glyph + truncated inserted text. Distinct from notice/error by a green
  left accent, not background tint (HUD bg stays `--hud-bg`):

```
┌─────────────────────────────────────────┐
│ ✓  "Let's ship the release on Friday."   │   ← green ✓, --hud-bg, fades after ≤1.5s
└─────────────────────────────────────────┘
```

- `hudState.ts`: add `case 'inserted': return ✓ + ellipsized message`. Truncate to ~48 chars
  with `text-overflow: ellipsis` (the pill already clips, `styles.css:604`).
- Respects reduced-motion (§4.6): with `prefers-reduced-motion`, skip the slide, keep the
  fade, keep the 1.5 s dwell.
- IPC: one added enum variant on `Status` (Rust + `types.ts`), mirrored per 00 §8.3. This is
  the only state change here that crosses IPC.

### 3.5 HUD notice vs error differentiation — assessment (UX-34)

Today they differ **only by background tint**: `.hud-error` red `rgb(120 24 24/94%)`
(`styles.css:612-614`), `.hud-notice` amber `rgb(112 84 16/94%)` (`:616-618`); same duration
(4 s, `pipeline.rs:37`), same layout, no icon. Color is the sole channel → fails for colorblind
users. Spec: add a **leading glyph** so severity survives without color —

- notice → `ⓘ` (info), error → `⚠` (warning), success → `✓` (§3.4).
- Keep the tints (now `--hud-bg-notice` / `--hud-bg-error` / `--hud-bg`), but the glyph is the
  primary signal. The glyph is `aria-hidden`; the meaning rides the label text + `aria-live`
  (§4.5).

### 3.6 Permission-denied states — assessment

Onboarding handles denial well: mic denied → "Open System Settings" (`Onboarding.tsx:83-87`),
accessibility not-granted → Grant + Open System Settings (`:107-119`). **Gap:** these states do
not exist _outside_ onboarding. When a user revokes Accessibility later, dictation silently
degrades to clipboard with only the transient HUD notice. Spec (small): an Output-page inline
callout when `!accessibility`: _"Accessibility is off — OpenFlow copies to the clipboard
instead of pasting. Grant it in System Settings."_ with an "Open System Settings" button. Reuse
the tip/callout primitive (§5).

### 3.7 Empty Last-result (UX-28)

Today the Last result card is absent before any dictation (`GeneralTab.tsx:179`), and tray
"Copy last dictation" no-ops silently (`tray.rs:84-92`). Spec (per 05 §3.4):

- Output page: show a muted placeholder card _"Your last dictation will appear here."_ instead
  of nothing.
- Tray: when `last_result()` is `None`, either disable the item or flash a HUD notice _"No
  dictation yet."_ on click.

---

## 4. Accessibility pass

### 4.1 Keyboard navigation map — settings window

Tab order (top → bottom, left → right):

```
[sidebar: Dictation → Modes → Models → Output → Dictionary → General → About]
   ↑ arrow-key roving within the tablist (§4.7); Tab moves OUT to content
        → [save banner Dismiss, if shown]
        → [first card control … last card control, in DOM order]
```

Fixes required:

- **Cmd+W / Esc closes the window** (UX-14). Today closing is mouse-only (red traffic-light,
  `main.rs:92-104`). Register a Cmd+W accelerator (and Esc) → same hide path. Does not touch the
  HUD invariant.
- **Sidebar is a tablist** (UX-31, §4.7) with roving tabindex and Left/Right arrows.
- **No focus trap.** The only trap today is the HotkeyRecorder (§4.4) — fix it to release Tab.

### 4.2 ModesTab click-row vs radio — the accessible pattern (UX-05)

Problem (`ModesTab.tsx:52-76`): one row carries **two gestures** — radio `onChange` activates
(`:69`), row `div onClick` selects-for-edit (`:56`); the inner `<label>` stops propagation
(`:60-63`). To AT this is a bare radio with no name plus a clickable `div` with no role. The
two states (active vs editing) are invisible to a screen reader.

Accessible v2 pattern (keep the idiom per 00 §10, make it legible):

- The row is a **radio in a named radiogroup** for _activation_, and a separate **"Edit"
  affordance** for editing — two controls, two names, not one ambiguous row.
- Structure:
  ```
  <div role="radiogroup" aria-label="Active writing mode">
    <div class="mode-row">
      <input type="radio" aria-label={`Use ${mode.name}`} … />
      <button class="mode-edit" aria-label={`Edit ${mode.name}`}>{mode.name}</button>
      {active && <span class="badge badge-ok">Active</span>}
      {usesLlm && <span class="badge">AI</span>}
    </div>
  </div>
  ```
- The mode name becomes a real `<button>` (the edit trigger), so it is keyboard-focusable and
  announced as "Edit Email, button". The radio is announced as "Use Email, radio, 1 of 5".
- Visible legend under the list (UX-05): _"Click the circle to switch modes. Click a name to
  edit it."_ Add an **"Active"** pill on the active row (separate from the radio dot) and the
  `mode-selected` background marks the edit target.

### 4.3 Toggle — VoiceOver (UX-30)

Renders today: `<button role="switch" aria-checked={checked} aria-label={label ?? 'toggle'}>`
(`Toggle.tsx:11-16`). The `role="switch"` and `aria-checked` are **correct and good** — credit
that. The only flaw is the `'toggle'` fallback label. Fix: make `label` a **required** prop
(TS-enforced), drop the fallback — every toggle then has a real accessible name. All current
call sites already pass one (`GeneralTab.tsx:167,173`, `ModesTab.tsx:111`).

### 4.4 HotkeyRecorder — screen-reader record/announce/escape (UX-03)

Today (`HotkeyRecorder.tsx:14-52`) it is the worst a11y defect: a capture-phase listener
`preventDefault/stopPropagation` on **every** key (`:17-18`), no `aria-label`, no role beyond
`<button>`, no live region, and Tab is swallowed so focus is trapped (only Esc or blur exits;
the "Esc to cancel" lives only in the `title` tooltip, `:48`). Spec the accessible pattern:

- **Button name** carries current value + instruction:
  `aria-label={`${title} shortcut, currently ${formatAcceleratorMac(value)}. Activate to record a new one.`}`
- **On entering record mode:** render a _visible_ helper line (not just `title`): _"Press a
  shortcut, or Esc to cancel."_ and announce it via an `aria-live="assertive"` region.
- **On capture:** announce the result — `aria-live` reads _"Recorded ⌥Space"_ (or _"Cancelled"_
  on Esc).
- **Escape hatch for keyboard users:** treat **Tab as cancel** (exit record mode and let focus
  move) so the user is never trapped (UX-03 fix d). Esc cancels (already does, `:19-21`).
- Add the helper as `aria-describedby` on the button.

### 4.5 Contrast — computed ratios for actual tokens

WCAG AA: body/UI 4.5:1, large text (≥18.66px or ≥14px bold) 3:1. Ratios below are computed
sRGB relative-luminance values against the stated background (verified, not estimated).

| Token pair                                           | Context                          | Light      | Dark                                | Verdict                                                       |
| ---------------------------------------------------- | -------------------------------- | ---------- | ----------------------------------- | ------------------------------------------------------------- |
| `--text` `#1d1d1f` on `--bg` `#f5f5f7`               | body                             | 16.0:1     | 15.4:1                              | pass                                                          |
| `--text-dim` `#6e6e73` on `--panel` `#fff`           | hints, card h2                   | 5.07:1     | 4.98:1                              | pass                                                          |
| `--text-dim` `#6e6e73` on `--bg` `#f5f5f7`           | hints on page bg                 | **4.66:1** | 5.4:1                               | pass (light margin is thin — do not lighten `--text-dim`)     |
| `--accent` `#5b5bd6` text on `--panel` `#fff`        | links, active model title `:392` | 5.37:1     | (`#7c7cf0` on `#2a2a2d`) 4.6:1      | pass                                                          |
| `--accent-text` `#fff` on `--accent` `#5b5bd6`       | primary buttons, active sidebar  | 5.37:1     | (`#fff` on `#7c7cf0`) **3.5:1**     | pass light; **dark 3.5:1 FAILS body 4.5** (passes large only) |
| `--danger` `#d62b2b` on `--panel` `#fff`             | form errors, btn-danger          | 4.95:1     | (`#ff6961` on `#2a2a2d`) 4.0:1      | pass light; **dark FAILS** (12px hint is body)                |
| `--ok` `#1f8a4c` on `--panel` `#fff`                 | `badge-ok` (11px)                | **4.38:1** | (`#5dd28a` on `#2a2a2d`) 6.4:1      | **light FAILS** body 4.5                                      |
| `--text-dim` on `--chip` `#e8e8ed`                   | default badge (11px)             | **4.15:1** | (`#98989d` on `#3a3a3e`) **4.32:1** | **FAILS both** (badge text is body)                           |
| `--hud-text` `#f5f5f7` on `--hud-bg` `rgb(28 28 32)` | HUD label                        | 14.6:1     | same (fixed)                        | pass                                                          |

Three real AA failures; the rest pass (notably `--accent` text at 5.37:1 and `--danger` on
white at 4.95:1 are **fine** — no change needed there). Fixes:

1. **Default badge** `--text-dim` on `--chip` (4.15 light / 4.32 dark) fails. Fix: badge text
   uses **`--text`** not `--text-dim` → 13.8:1 light; keep 11px. One-line CSS change (§5.4).
2. **`badge-ok`** `--ok` on white (4.38) fails at 11px. Fix: status badges (`badge-ok`,
   `badge-active`) bump to 12px (`--text-hint`), which makes 4.38 a near-large case, **and**
   darken the light `--ok` used in badges to `#187a40` (5.0:1) to clear body. Decorative badges
   stay 11px.
3. **`--accent-text` on `--accent` in dark** (3.5:1) fails body for `btn-primary` / active
   sidebar labels (≥13px medium = body). Fix: nudge the dark accent used as a _button/active
   background_ to `#6a6ae8` so white-on-it clears 4.5. (Text-colored `--accent` is fine in dark
   at 4.6:1 — only the filled-background case needs the nudge.)

The §1.2 `--danger:#c4241f` darken is a **dark-mode-safety + margin** choice, not strictly
required for light (the original `#d62b2b` already passes light at 4.95) — keep it for a
comfortable buffer and better dark legibility. `--text-dim` stays `#6e6e73`: it is at 4.66 on
`--bg` and lightening it would drop below 4.5.

### 4.6 Reduced motion (NEW)

No `@media (prefers-reduced-motion)` exists today — the HUD bars (`styles.css:633`), spinner
(`:647`), pulse (`:351`), and fades (`:599`) all animate unconditionally. Spec:

```css
@media (prefers-reduced-motion: reduce) {
  .hud-pill {
    transition: opacity var(--motion-hud) var(--ease);
    transform: none;
  }
  .hud-visible {
    transform: none;
  }
  .hud-bars span {
    transition: none;
  } /* hold at rest height */
  .hud-spinner {
    animation: none;
    border-top-color: transparent;
  } /* static ring */
  .hotkey-recording {
    animation: none;
  } /* drop pulse; keep accent border */
  .toggle,
  .toggle-knob {
    transition: none;
  }
}
```

The HUD still communicates state via label + glyph (§3.5) and the success dwell (§3.4) — motion
is decorative, so removing it loses nothing functional. The spinner becomes a static ring; pair
it with the existing label so "busy" is still legible.

### 4.7 HUD aria-live + sidebar tablist

- **HUD** (UX-29, UX-34): the pill is a separate always-on click-through webview. VoiceOver
  should hear state changes but the bars must stay decorative. Spec: wrap `.hud-label` (or the
  pill) in `aria-live="polite"` and `aria-atomic="true"`; keep `aria-hidden` on bars
  (`Hud.tsx:34`) and spinner (`:40`). "polite" (not "assertive") so it does not interrupt the
  app the user is dictating into. Errors could justify "assertive" — but since the HUD is an
  overlay on another app's focus, "polite" is the safe default; the inserted-text success and
  the on-screen app's own change carry the real confirmation.
- **Sidebar** (UX-31): `role="tablist"` on `<nav>`, `role="tab"` + `aria-selected` on each
  button (`App.tsx:37-47`), `role="tabpanel"` + `aria-labelledby` on `.content`. Roving
  tabindex + Left/Right arrow handling. Icons (§5, 03 §4) get `aria-hidden` since the label is
  the name.

---

## 5. Component spec updates

Current contract → v2 contract for each primitive. Tokens consumed reference §1.2.

### 5.1 Row (`Row.tsx`)

- **Today:** `{ title, hint?, children }` → `.row` with `.row-text` (title + optional hint) +
  `.row-control`. No id wiring; the control's label association is implicit.
- **v2:** add optional `id` and `status?: 'default' | 'warn' | 'error'`. When `hint` is present,
  give it `id={`${id}-hint`}` and expect the control to `aria-describedby` it (the hint becomes
  a real description, not just visual). `status` tints the hint via `--warn` / `--danger` for
  state-aware hints (UX-13, UX-22, §3.6). Tokens: `--space-4` row padding, `--border` top rule,
  `--text-title`/`--weight-medium` title, `--text-hint`/`--text-dim` hint.

### 5.2 Toggle (`Toggle.tsx`)

- **Today:** `{ checked, onChange, label? }` → `<button role="switch" aria-checked
aria-label={label ?? 'toggle'}>`. (Roles correct.)
- **v2:** `label` **required** (drop `'toggle'` fallback, UX-30). Add `disabled?` →
  `aria-disabled` + 0.5 opacity. Add `:focus-visible` ring (inherits global `:70-76`). Tokens:
  `--chip` off, `--accent` on (`.toggle-on`), `--shadow-knob`, `--motion-fast`. No structural
  change — it is already a good switch.

### 5.3 HotkeyRecorder (`HotkeyRecorder.tsx`)

- **Today:** `{ value, onChange }`; click → record; capture-phase swallows all keys; Esc
  cancels; `title` tooltip only; no a11y name. (Trap — UX-03.)
- **v2 contract** (full fix, §4.4): same props + internal `recording` state. Adds: descriptive
  `aria-label` with current value, visible helper line while recording, `aria-live="assertive"`
  announcer, **Tab cancels** (no trap), `aria-describedby` the helper. States: `idle` (shows
  formatted accelerator), `recording` (shows "Press shortcut…", pulse border — dropped under
  reduced-motion). Tokens: `--chip`/`--border`/`--radius-sm`, `--accent` border when recording.

### 5.4 badge (`.badge`)

- **Today:** `.badge` chip, `--chip` bg, `--text-dim` text, 11px (`styles.css:240-256`); variants
  `badge-ok` (`--ok` text), `badge-muted` (0.7 opacity).
- **v2:** badge text uses **`--text`** not `--text-dim` (contrast fix #1, §4.5). Add semantic
  variants used by the specs: `badge-active` (the Modes "Active" pill, `--ok` bg-tint),
  `badge-local` / `badge-cloud` (Models AI-profile local/cloud derivation, 03 §F5/F6 — green vs
  neutral). Status badges (`badge-ok`, `badge-active`) bump to 12px (`--text-hint`) for AA.

### 5.5 btn variants (`.btn`)

- **Today:** `.btn` (`--chip` bg, `--border`, `--radius-sm`), `:hover filter:brightness(0.96)`,
  `:disabled opacity 0.5`; `.btn-primary` (`--accent`), `.btn-quiet` (transparent, `--text-dim`),
  `.btn-danger` (`--danger` text).
- **v2:** replace `filter:brightness` hover with explicit `--accent-hover` (primary) and a
  `--chip` darken for default — `filter:brightness` is invisible in dark mode where chips are
  already dark. `.btn-danger` text uses the new `#c4241f` (margin; light already passed). Ensure all carry the
  global `:focus-visible` ring. Add a `.btn-sm` size for inline row actions (Retry, Copy raw)
  using `--space-2`/`--space-3` padding.

### 5.6 card (`.card`)

- **Today:** `.card` `--panel` bg, `--border`, `--radius-lg`, `--space-7`/`--space-8` padding;
  `.card h2` 13px uppercase `--text-dim` (`styles.css:146-159`).
- **v2:** add `--shadow-card` (subtle, both themes); add an optional **card subhead** slot (one
  `.row-hint` line under `h2`) for the Modes "Output styles…" subtitle (UX-06) and page
  intros (05 §4). `h2` stays uppercase but should carry the page/section name accessibly
  (`<h2>` is already a heading — good).

### 5.7 NEW — sidebar icon item (03 §4 "icons added")

03 adds icons to the seven sidebar items. Spec a primitive:

```
<button role="tab" aria-selected={active} class="sidebar-item">
  <svg class="sidebar-icon" aria-hidden …/>   <!-- 16px, currentColor -->
  <span>{label}</span>
</button>
```

- Icon inherits `currentColor` so it flips with `.sidebar-active` (white on `--accent`).
- 16px box, `--space-4` gap to label, no separate color token (uses text color).
- Icons are decorative (`aria-hidden`); the label is the accessible name.
- No icon library (00 §8.5) — inline SVG paths, one per page (Dictation: waveform; Modes:
  sliders; Models: chip; Output: arrow-out; Dictionary: book; General: gear; About: info).

### 5.8 NEW — tip / callout block (05 §2, 03 §4)

A non-modal inline notice used for: no-model callout (UX-01), permission-off callout (§3.6),
feature tips (05 §2). Spec:

```
<aside class="callout callout-{info|warn}" role="note">
  <svg class="callout-icon" aria-hidden/>
  <div class="callout-body">
    <p>{message}</p>
    {action && <button class="btn btn-sm">{action}</button>}
  </div>
  {dismissible && <button class="btn-quiet" aria-label="Dismiss">×</button>}
</aside>
```

```css
.callout {
  display: flex;
  gap: var(--space-4);
  align-items: flex-start;
  border: 1px solid var(--border);
  border-left-width: 3px;
  border-radius: var(--radius-md);
  padding: var(--space-4) var(--space-6);
  background: var(--panel);
  font-size: var(--text-hint);
}
.callout-info {
  border-left-color: var(--accent);
}
.callout-warn {
  border-left-color: var(--warn);
  background: var(--warn-bg);
}
.callout-icon {
  width: 16px;
  flex-shrink: 0;
  color: var(--accent);
}
.callout-warn .callout-icon {
  color: var(--warn);
}
```

- Variants: `info` (accent left bar — feature tips), `warn` (amber, actionable — missing model /
  permission). Optional action button and dismiss. `role="note"`; dismiss button is labeled.
- Reuses the existing `.privacy-note` left-bar idiom (`styles.css:291-297`) generalized — that
  pattern is already in the codebase, so this is a consolidation, not new machinery.

---

## 6. Before / after — the three worst screens

Chosen from this review by impact: (1) **General tab overload** — one page doing six jobs, the
root IA problem (03 D2); (2) **ModesTab dual-gesture row** — the canonical confusion (UX-05);
(3) **HUD notice/error** — color-only severity + no success moment (UX-34, UX-08, §3.4).

### 6.1 General tab → split (03 §2/§3)

**Before** (`GeneralTab.tsx` — one scroll holding hotkeys, models, language, output, last
result; four `<section class="card">`, `:44/74/152/180`):

```
┌ General ─────────────────────────────────────────────┐
│ HOTKEYS                                                │
│  Dictation              [⌥Space]                       │
│  Dictation style        [Hold to talk ▾]               │
│  Rewrite selection      [⌥⇧Space]                      │
├────────────────────────────────────────────────────── │
│ SPEECH RECOGNITION                                     │
│  ◉ Base (English) multilingual   148.0 MB  [installed] │
│  ○ Small (English)               488.0 MB  [Download]  │
│  …                                                      │
│  Spoken language        [Auto-detect ▾]                │
│   "English-only models ignore this."                   │
├────────────────────────────────────────────────────── │
│ OUTPUT                                                  │
│  Insert method          [Paste ▾]                      │
│  Restore clipboard      [●——]                          │
│  Launch at login        [●——]                          │
├────────────────────────────────────────────────────── │
│ LAST RESULT  (only if a dictation happened)            │
│  "…"                                          [Copy]    │
└────────────────────────────────────────────────────────┘
```

**After** (four pages; the user lands on Dictation, which now does exactly one job):

```
Sidebar                  ┌ Dictation ─────────────────────────────┐
 🎙 Dictation  ◀ default  │ HOTKEYS                                  │
 🎚 Modes                 │  Dictation              [⌥Space]         │
 🧩 Models                │  When I press the hotkey [Hold to talk ▾]│  ← was "Dictation style"
 ↗ Output                 │  Polish selection       [⌥⇧P]   (Refine) │
 📖 Dictionary            │  Rewrite selection      [⌥⇧Space]        │
 ⚙ General                │   "…Needs an AI profile."                │
 ⓘ About                  │ SPEECH                                   │
                          │  Spoken language  [Auto-detect ▾]        │
                          │   (disabled for EN-only model + hint)    │
                          │ AFTER TRANSCRIBING                       │
                          │  Refine with AI         [●——]            │
                          └──────────────────────────────────────────┘
   Models page  → SPEECH RECOGNITION (model list) + AI PROFILES
   Output page  → Insert method · Restore clipboard · Last result
   General page → Launch at login · Appearance · Welcome tour · Troubleshooting
```

Caption — what changed and why: the six-job page splits along 03's recurring questions ("what
runs my speech/AI" → Models; "where did my text go" → Output). Each card now owns one concern.
Copy fixes ride along: "Dictation style" → "When I press the hotkey" (UX-16); EN-only language
select disabled with an actionable hint (UX-22); Rewrite gains its AI-dependency hint (UX-09);
the `refineAfterDictation` switch lands under "After transcribing" (03 §3). Sidebar gains icons
(§5.7) and tablist semantics (§4.7). Tokens: unchanged card/row styling — pure reorganization
plus the new `--space`/`--radius` names.

### 6.2 ModesTab dual-gesture list (UX-05)

**Before** (`ModesTab.tsx:52-76` — radio activates, row-click edits, same row, no labels):

```
┌ Modes ───────────────────────────────────────┐
│ "The active mode shapes how transcripts…"      │
│  ○ Standard          [AI] [built-in]           │  ← radio = active (which?)
│  ● Email             [AI] [built-in]           │  ← row bg = editing (which?)
│  ○ Notes             [AI] [built-in]           │
│  ○ Literal               [built-in]            │
│  [ New mode ]                                   │
└────────────────────────────────────────────────┘
   (active vs editing are two invisible, conflated states)
```

**After** (named radiogroup + edit buttons + explicit "Active" pill + legend, §4.2):

```
┌ Modes ─────────────────────────────────────────────────┐
│ Output styles — how your dictation is written.          │  ← subhead (UX-06)
│ ┌ radiogroup: "Active writing mode" ───────────────────┐│
│ │ ○  Standard            [AI] [built-in]                ││
│ │ ◉  Email   ‹Active›     [AI] [built-in]   ◀ editing   ││  ← green Active pill + sel bg
│ │ ○  Notes               [AI] [built-in]                ││
│ │ ○  Literal             [built-in]                     ││
│ └───────────────────────────────────────────────────────┘│
│ Click the circle to switch modes. Click a name to edit.  │  ← legend (UX-05)
│ [ New mode ]   [ Browse templates… ]                     │  ← (03 §2, 06)
└──────────────────────────────────────────────────────────┘
```

Caption: the row keeps the established radio-selects-active + click-to-edit idiom (00 §10) but
makes both states legible and accessible — the radio gets `aria-label="Use {mode}"`, the name
becomes an `aria-label="Edit {mode}"` button, an "Active" pill marks the live mode independent
of the radio dot, and a one-line legend names both gestures. Tokens: new `badge-active`
(§5.4, `--ok` tint), `--chip` edit-row background (unchanged). VoiceOver now hears "Use Email,
radio, 2 of 4" and "Edit Email, button" instead of an unnamed radio + clickable div.

### 6.3 HUD notice / error (UX-34, UX-08, §3.4)

**Before** (`hudState.ts:11`, `styles.css:612-618` — color-only severity, no success, one label
for three refining ops):

```
recording     ░ ▒ █ ▒ ░  Listening…
refining        ◐  Polishing…            ← same word for dictation-cleanup AND rewrite
notice (amber)     Copied to clipboard — press ⌘V…
error  (red)       AI provider error: provider timed out after 30s   ← dev prefix, color-only
(success)       (nothing — HUD just fades out)
```

**After** (job-aware labels + leading glyph + success flash + sentence-case copy):

```
recording     ░ ▒ █ ▒ ░  Listening…                       (Rewrite: "Listening for instruction…")
refining        ◐  Cleaning up…            (dictation)   /  ◐ Rewriting…   (selection)   ← UX-08
inserted (≤1.5s) ✓  "Let's ship on Friday."               ← NEW success flash, green ✓ (§3.4)
notice          ⓘ  Copied to clipboard — press ⌘V to paste.   ← glyph + tint (§3.5)
error           ⚠  Couldn't reach the AI profile — try again.  ← glyph; no "x error:" prefix (UX-20)
```

Caption: three fixes in the feedback surface. (1) `refining` becomes job-aware — "Cleaning up…"
for dictation vs "Rewriting…" for selection (UX-08, pure `hudState.ts`). (2) A leading glyph
(`✓`/`ⓘ`/`⚠`) makes severity survive without color for colorblind users (UX-34), backed by
`aria-live="polite"` on the label (§4.7). (3) A ≤1.5 s success flash shows the inserted text so
the user gets confirmation instead of a silent fade (§3.4, 03 §F1). Copy is sentence-cased and
the `AppError` "x error:" prefix is stripped before display (UX-20). Tokens: `--hud-bg-notice`,
`--hud-bg-error`, new `inserted` status with green `✓`; HUD stays click-through and
content-only (00 §8.2). Under reduced motion the slide is dropped, the fade and the 1.5 s dwell
remain (§4.6).

---

## 7. Implementation order (PR-sized chunks)

Grouped so each PR is reviewable and the dependency on 03's IA reshuffle is explicit.

| #   | PR                                                                                                                                                                                                                                                                        | Files                                                                                                                                                         | Size | Blocks / blocked by                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Tokens + dark-mode override** — replace `:root`/dark block with §1.2; add `appearance` field (Rust + `types.ts`), General → Appearance row; wire `dataset.theme`; contrast fixes (#1–3, §4.5)                                                                           | `styles.css`, `settings.rs`, `packages/core/src/types.ts`, `GeneralTab.tsx`(→General page), App init guard                                                    | M    | **Independent** of IA; lands first. Contrast token nudges are pure CSS. IPC mirror per 00 §8.3.                                                                             |
| 2   | **Copy sweep** — all §2 string changes that don't move surfaces: HUD labels (`hudState.ts`), notices/errors (`pipeline.rs`), `AppError` prefix strip (`error.rs`), tray header + "Copy last dictation" (`tray.rs`), hint rewrites in tabs                                 | `hudState.ts`, `pipeline.rs`, `error.rs`, `tray.rs`, `GeneralTab.tsx`, `ModesTab.tsx`, `DictionaryTab.tsx`, `Onboarding.tsx`                                  | S–M  | **Independent.** Most are one-line. UX-08/20 quick wins live here.                                                                                                          |
| 3   | **State coverage** — model download-fail inline state + Retry (UX-15), HUD success flash + `inserted` status (§3.4), notice/error glyphs (§3.5), test-result persistence (UX-33), empty Last-result + tray no-op feedback (UX-28), save banner `role="alert"`             | `GeneralTab.tsx`/Models page, `Onboarding.tsx`, `Hud.tsx`, `hudState.ts`, `pipeline.rs`(`inserted` enum), `types.ts`, `ProviderTab.tsx`, `tray.rs`, `App.tsx` | M    | `inserted` status crosses IPC (mirror). Download-fail and test-fix are independent; success flash is independent of IA.                                                     |
| 4   | **Accessibility** — sidebar tablist + roving arrows (UX-31), Cmd+W/Esc close (UX-14), HotkeyRecorder full fix (UX-03), Toggle required label (UX-30), ModesTab radiogroup pattern (UX-05), HUD `aria-live` (§4.7), reduced-motion block (§4.6)                            | `App.tsx`, `main.rs`(accelerator), `HotkeyRecorder.tsx`, `Toggle.tsx`, `ModesTab.tsx`, `Hud.tsx`, `styles.css`                                                | M–L  | ModesTab a11y pairs with its visual redesign (6.2). Tablist semantics assume the 7-page sidebar → **lightly coupled to IA (PR 5)**; can land on current 5 tabs then extend. |
| 5   | **Components + IA reshuffle** — split General into Dictation/Output/Models/General pages (03 §2/§3), add Modes subhead + templates entry, sidebar icon item (§5.7), tip/callout primitive (§5.8), no-model + permission-off callouts (UX-01, §3.6), card subhead + shadow | `App.tsx`(routing), new/renamed `tabs/*.tsx`, new `components/Callout.tsx`, `components/SidebarItem.tsx`, `styles.css`, `Row.tsx` props                       | L    | **This IS 03's reshuffle** — the structural PR. PRs 1–4 are designed to land before or independently of it; the callout primitive (§5.8) is consumed by UX-01/§3.6 here.    |

Sequencing note: PRs **1, 2, 3** are independent of the IA reshuffle and deliver the highest
ratio of fixed audit findings per line (tokens, the whole copy sweep, the success flash). PR
**5** is the IA reshuffle (03) and should land after 1–3 so the new pages inherit the
consolidated tokens and corrected copy. PR **4** straddles: the HotkeyRecorder, Toggle, Cmd+W,
and reduced-motion fixes are IA-independent and can ship anytime; only the _7-page_ tablist
wiring benefits from PR 5 landing first (it works on 5 tabs too). Nothing here adds a
dependency or a component library (00 §8.5); every change is CSS, React markup, copy, or one
additive `appearance` field + one additive `inserted` status across the IPC mirror.
