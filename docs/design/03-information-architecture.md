# 03 — Information architecture for v2

> **Superseded (feat/UX-redesign).** The shipped app splits into **two windows**, not one
> sidebar: an **App window** (Home · Library · Transform · Scratchpad · ⚙ Settings) and a
> separate **Settings window** (Dictation · Speech · AI · General · About · ‹ Velata). The
> "one switchable concept" below is no longer the **Mode** — modes are deleted; the single
> concept is the **Prompt** (a named instruction + shortcut; the one built-in is Polish),
> managed on the App's Transform page. Insights is no longer a sidebar page or an opt-in — it
> is always-on counts/dates surfaced in the Home header (never words or audio). The grouped-card
> page-anatomy rules in §4 still apply. The rest is kept for its rationale.
>
> **Update 2026-06-13 (Wispr-parity build).** A two-section sidebar now supersedes the
> seven-page IA defined below: **Features** (Home · Insights · Dictionary · Snippets · Style ·
> Transforms · Scratchpad) and **Settings** (Dictation · Modes · Models · Output · General ·
> About). The page-anatomy rules in §4 (grouped cards; one flat row per feature; one control
> and at most one hint line) still apply to every page. This grouping was decided during the
> Wispr-parity build; the rest of this document is kept for its rationale.

Status: design proposal. Written 2026-06-11 against `main` (cd63494) + the in-flight Refine
work (`docs/REFINE.md`). Baseline facts and vocabulary: `00-current-state.md`. This document
is the keystone: 04–09 follow its structure and naming. Companion docs: 01 (competitive
evidence), 02 (audit findings the structure must fix).

## 1. The two structural decisions

### D1. Modes are the one switchable bundle

The brief asks for "profiles" (Developer, Student, Writer…) that bundle LLM, STT,
instructions, output, and shortcuts — switchable in one click. Velata already has a
switchable bundle: the **Mode**. Adding a second switchable concept ("profile") on top of
modes would force every user to learn a 2-axis matrix (which mode × which profile is active?)
and double every "why did it write that?" debugging path. Superwhisper — the best-regarded
customization model in this category — has exactly one such concept, also called modes, and
bundles STT model + AI model + prompt + output behavior inside it.

**Decision:** extend `Mode` with optional, inherit-by-default overrides instead of adding a
new concept.

```
Mode v2 = {
  id, name, builtIn,
  prompt,                       // the instruction (unchanged)
  usesLlm,                      // wants AI refinement (unchanged)
  aiProfileId:  string | null,  // null = use the globally active AI profile
  sttModelId:   string | null,  // null = use the global speech model
  language:     string | null,  // null = use the global spoken language
  hotkey:       string | null,  // optional: press to dictate directly in this mode
}
```

- Every override is `null` by default → a fresh custom mode behaves exactly like today's.
  Beginners never see the matrix; power users get the full bundle.
- The persona presets from the brief (Email, Coding, Meeting Notes, Translation, Slack,
  Academic…) ship as **mode templates** — a gallery in the Modes page, not new machinery.
  Details: `06-custom-instructions.md` (templates) and `07-profiles.md` (bundling and
  switching UX).
- **"AI profile" keeps meaning an LLM connection** (REFINE.md), never a persona. UI copy says
  "mode" for the bundle. This preserves the canonical vocabulary table in 00 §7.

### D2. Concrete pages, not abstract categories

The brief mandates organizing around Input / Processing / Models / Output / Personalization.
Those are the right _dimensions_ for completeness checking — but they are the wrong _labels_
for navigation. Users do not look for "Processing"; they look for "Modes" (the word the tray
already taught them) or "Dictation" (the thing they do). macOS System Settings, Raycast, and
Superwhisper all use concrete nouns. The five dimensions map onto six concrete pages:

| Brief dimension | Lands in                                                                                                          |
| --------------- | ----------------------------------------------------------------------------------------------------------------- |
| Input           | **Dictation** (hotkeys, hold/toggle, spoken language, feedback)                                                   |
| Processing      | **Modes** (instructions/refinement/translation/rewriting behavior) + the "Refine with AI" switch on **Dictation** |
| Models          | **Models** (speech models & engines; AI profiles — local/cloud)                                                   |
| Output          | **Output** (insert method, clipboard, formatting, recovery)                                                       |
| Personalization | **Modes** (custom modes, templates, per-mode hotkeys), **Dictionary**, hotkeys on **Dictation**                   |

## 2. Sitemap

```
Velata
├── Menu-bar tray  (quick-switch surface — no configuration)
│   ├── Mode               ○ Standard  ● Email  ○ Notes  ○ Literal  ○ <custom…>
│   ├── Refine with AI     ✓ (mirrors Dictation → Refine with AI; in-flight item)
│   ├── ────────────
│   ├── Copy Last Result
│   ├── Settings…                       (opens window, last-used page)
│   └── Quit Velata
│
├── HUD pill  (feedback only; never configurable from itself)
│   states: listening / listening-for-instruction / transcribing / refining(job-aware)
│           / inserting / notice / error          (specs: 09-ux-polish.md)
│
├── Onboarding  (first run + re-runnable from General; redesign: 04-onboarding.md)
│
└── Settings window  (sidebar, macOS Settings idiom)
    ├── Dictation                        ← default page
    │   ├── Hotkeys              [card: all four bindings in one place]
    │   │   ├── Dictation                ⌥Space        (recorder)
    │   │   ├── Dictation style          Hold to talk ▾ (hold / press-to-toggle)
    │   │   ├── Polish selection         ⌥⇧P           (recorder; in-flight feature)
    │   │   └── Rewrite selection        ⌥⇧Space       (recorder)
    │   ├── Speech
    │   │   ├── Spoken language          Auto-detect ▾  (hint: per-mode override exists)
    │   │   └── Microphone               System default ▾   (future row — reserved)
    │   ├── After transcribing
    │   │   └── Refine with AI           [toggle]  "Polish transcripts with your active
    │   │         mode and AI profile. Off = fast rules-based cleanup, no network."
    │   └── Feedback
    │       ├── Sounds                   [toggle]   (roadmap: start/stop cues)
    │       └── Menu-bar recording icon  [toggle]   (roadmap)
    │
    ├── Modes
    │   ├── Mode list            [radio = active; click = edit; "New mode" / "Browse templates…"]
    │   ├── Template gallery     (sheet: Email, Meeting Notes, Slack, Coding, Academic,
    │   │                         Translation, … — spec: 06)
    │   └── Mode editor          Name · Prompt · Uses AI · Advanced (collapsed):
    │                            AI profile ▾ inherit · Speech model ▾ inherit ·
    │                            Language ▾ inherit · Mode hotkey (recorder) ·
    │                            Duplicate / Delete / Export / Import   (spec: 06, 07)
    │
    ├── Models
    │   ├── Speech recognition   [model list: radio = active, download/delete, size,
    │   │                         multilingual badge; future: engine column — spec: 08]
    │   └── AI profiles          [list: radio = "No AI" | profiles, local/cloud badge;
    │                             editor: name/provider/URL/key/model/timeout/test;
    │                             Show in Finder = import/export   — in-flight design,
    │                             re-homed here; spec: 08]
    │
    ├── Output
    │   ├── Insert method        Paste into the active app ▾ / Copy to clipboard only
    │   ├── Restore clipboard    [toggle]
    │   └── Last result          [card: final text, raw transcript, Copy — recovery]
    │
    ├── Dictionary
    │   ├── Add entry            from → to
    │   ├── Entry list
    │   └── Import / Export CSV  (roadmap item, slot reserved)
    │
    ├── General
    │   ├── Launch at login      [toggle]
    │   ├── Appearance           System ▾ (system/light/dark — spec: 09)
    │   ├── Welcome tour         [Show again]   (re-runs onboarding)
    │   └── Troubleshooting      Open logs folder · Reset all settings…
    │
    └── About
        └── version · data folder · config path · licenses · links
```

Seven sidebar items (was five). The two additions earn their place: Output and Models each
answer a distinct recurring question ("where did my text go?" / "what runs my speech and AI,
and is it local?") that today hides inside General and AI Provider.

## 3. What moves where (migration map)

| Today (committed)                            | v2 home                                                                                                                                  |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| General → Hotkeys card                       | Dictation → Hotkeys (+ Polish row, in flight)                                                                                            |
| General → Speech recognition model list      | Models → Speech recognition                                                                                                              |
| General → Spoken language                    | Dictation → Speech                                                                                                                       |
| General → Output card (insert/restore/login) | Output (insert, restore) · General (login)                                                                                               |
| General → Last result card                   | Output → Last result                                                                                                                     |
| Modes tab                                    | Modes (+ templates, + advanced overrides)                                                                                                |
| Dictionary tab                               | Dictionary (unchanged, + import/export slot)                                                                                             |
| AI Provider tab → in-flight Refine tab       | Split: profiles list/editor → Models → AI profiles; `refineAfterDictation` toggle → Dictation → After transcribing (tray item unchanged) |
| About tab                                    | About (unchanged)                                                                                                                        |

Note on the in-flight Refine tab: everything REFINE.md designs (profile list idiom, editor,
local/cloud badge, Show in Finder, empty state, "No AI" radio) is kept verbatim — only its
_address_ changes from a tab named after the engine verb to the Models page, because task 8
adds STT engines that need the same provider-management UX and the symmetry should be visible
in one place. If Refine ships as its own tab first, the re-home is a low-cost rename later;
nothing else in this IA depends on it.

The `refineAfterDictation` toggle moves to Dictation because of what it actually gates: it is
a dictation-pipeline switch (Polish and Rewrite deliberately ignore it, per REFINE.md). On a
"Refine" tab next to the profile list it reads as a second, redundant "No AI" control; next
to "After transcribing" it reads as what it is.

## 4. Navigation and layout model

- **Sidebar** (current idiom, kept): fixed item order as in §2; icons added per 09. Default
  page: Dictation. The window remembers the last-open page per session only.
- **Page anatomy** (kept from REFINE.md's principle): grouped cards; every feature is one
  flat row — name that states what it does, one control, at most one short hint line. No
  nested indentation, no progressive sub-forms except the mode editor's single "Advanced"
  collapse.
- **Tray** stays the quick-switch surface: anything switchable-in-one-click (mode, refine
  toggle) appears there; nothing configurable does. Deep links: tray "Settings…" opens the
  window; HUD error notices that name a fix ("add an AI profile in Settings") should open the
  relevant page when the settings window next opens (stored target, no new window types).
- **No settings search in v2.** Seven pages with ≤ 4 cards each is below the threshold where
  search pays for itself; revisit when per-app rules and history land (roadmap P3).
- **Scaling rule:** a new feature must (a) join an existing card as one row, (b) add a card to
  the page matching its brief-dimension, or (c) — only if it is switchable — add a tray item.
  New sidebar items require a new recurring user question, like Output and Models above.

Where announced roadmap features will land (so the IA does not break later):

| Future feature            | Home                                               |
| ------------------------- | -------------------------------------------------- |
| Per-app modes             | Modes → "App rules" card (app → mode table)        |
| Opt-in history            | General → toggle; enabling reveals a History page  |
| Command mode              | Dictation → new card                               |
| Spoken punctuation/casing | Output → Formatting card (per-mode override later) |
| Audio cues / VAD tuning   | Dictation → Feedback / Speech                      |
| Keychain key storage      | Models → AI profile editor (storage row)           |
| Alternative STT engines   | Models → Speech recognition (engine column, 08)    |
| Translation               | Modes → template + per-mode language override      |
| Fn push-to-talk           | Dictation → Hotkeys (recorder learns Fn, opt-in)   |
| Dictionary CSV round-trip | Dictionary → Import/Export                         |
| Settings search           | Window chrome, when page count grows               |

## 5. Primary user flows

Notation: `tray>` menu-bar, `S:` settings page, `HUD:` overlay, `⌨` global hotkey.

**F1 — First dictation (cold start).** Onboarding (04) → lands with base.en installed, mode
Standard, no AI profile → user clicks into any app, holds ⌥Space → `HUD: Listening…` →
release → `HUD: Transcribing… → Inserting…` → text appears → HUD success flash shows the
inserted text briefly (new, 04/09). Zero settings visits. Target < 60 s from first launch.

**F2 — Switch mode.** `tray> Mode > Notes` — 2 clicks, no window. Alternative: per-mode
hotkey (D1) — 0 clicks. The next HUD label shows the mode name ("Listening — Notes", 09) so
the switch is confirmed where the user is looking.

**F3 — Create a custom mode.** S:Modes → "Browse templates…" → pick "Slack" → it copies into
an editable custom mode, selected for editing → tweak prompt → done (auto-saved). 4 clicks +
typing. From-scratch path stays: "New mode".

**F4 — Fix a misheard word.** Dictation inserted "open flow" → S:Dictionary → from/to → Add.
3 clicks + typing. (Faster path — "Add correction" action on the Last result card pre-filled
with the raw/final text — spec'd in 05 as a discoverability quick win.)

**F5 — Add local AI (Ollama).** S:Models → AI profiles → "New profile" → provider Ollama
(URL prefilled) → "List installed models" → pick chip → Test connection → radio-select it →
done. One page, no manual typing if Ollama is running. Empty-profile-list state links here
from wherever AI is mentioned (05).

**F6 — Add cloud AI (BYO key).** Same as F5 with provider "OpenAI-compatible"; editor shows
the cloud privacy note (text-only leaves the machine) before the key field. The badge in the
list flips to `cloud` derived from the URL.

**F7 — Rewrite selection.** Select text anywhere → hold ⌥⇧Space → `HUD: Listening for
instruction…` → speak "make it shorter" → release → `HUD: Rewriting… → Inserting…` →
selection replaced. Failure: error notice, selection untouched.

**F8 — Polish selection (in flight).** Select → tap ⌥⇧P → `HUD: Polishing selection… →
Inserting…`. No voice. Without a profile: error notice names the fix and deep-links (S:Models).

**F9 — Recover output.** Paste failed or wrong app focused → `HUD: "Copied to clipboard —
press ⌘V"` (in-flight outcome model) → or later: `tray> Copy Last Result` / S:Output → Last
result → Copy.

**F10 — Temporarily disable AI.** `tray> Refine with AI` (uncheck) — 2 clicks, profile and
modes untouched. Same switch lives at S:Dictation → After transcribing.

**F11 — Better accuracy.** S:Models → Speech recognition → download small.en/large → radio.
Hint text states the tradeoff in seconds, not adjectives (09).

**F12 — Re-run onboarding.** S:General → Welcome tour → Show again. (New; fixes a dead end —
today `onboardingCompleted` can only be reset by editing JSON.)

## 6. Settings/IPC impact of this IA

The IA itself is a UI reorganization — `Settings` schema changes come only from D1 and are
specified in 07 (mode overrides; all optional, `null`-defaulted, additive serde fields) plus
two General items (appearance, re-run onboarding = writing `onboardingCompleted: false`).
No new persistence stores. The IPC mirror discipline (00 §8.3) applies to the mode shape.

## 7. Deliberately not doing

- A second switchable "profile/persona" concept beside modes (D1).
- Literal Input/Processing/Models/Output/Personalization tab names (D2).
- Settings search, command palette, or in-window tabs-within-tabs at this page count.
- Moving quick actions into the HUD (it must stay click-through — 00 §8.2).
- Any IA that requires accounts, sync, telemetry, or persistent history by default (00 §8).
