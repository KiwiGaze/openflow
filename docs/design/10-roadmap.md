# 10 — Roadmap: P0 → P3

Status: synthesis. Written 2026-06-11. Prioritizes everything proposed in 01–09 plus the
existing `ROADMAP.md`, against the audit evidence in 02 and the competitive evidence in 01.
Effort: S = hours–a day, M = days, L = a week or more. "Eng / Design" = engineering vs design
complexity. Every item cites its spec doc — nothing here is new design.

## 0. Sequencing spine

Four facts order everything:

1. **The in-flight Refine work ships first.** Profiles, `⌥⇧P` Polish, the refine toggle, and
   settings **v2** (REFINE.md) are mostly implemented and everything in 05–08 builds on them.
2. **The UX debt is discoverability and naming, not the core loop** (02 exec summary). The
   cheapest fixes have the highest first-run impact, so P0 is mostly copy, hints, and states.
3. **One settings bump per release.** 07 and 08 each assumed "the next bump." Resolved:
   **v3** = Mode v2 overrides + `transforms` + tips fields + `appearance` + `inserted` status
   (all additive, one no-op migration, ships with the 2.0 release). **v4** = engine-qualified
   `sttModelId` + `confirmedCloudSttEngines` (the only rewriting migration, ships with cloud
   STT engines).
4. **09's PR sequencing holds:** tokens (PR1) → copy sweep (PR2) → states (PR3) → a11y (PR4)
   land before or independently of the IA reshuffle (PR5), so the new pages inherit corrected
   tokens and copy.

```
Release A (1.x patch)   P0: quick wins, Esc, copy sweep, HUD success, download errors
Release B (1.x minor)   Refine lands · onboarding rebuild · discoverability top-5 ·
                        tokens/dark/a11y · LLM presets
Release C (2.0)         IA reshuffle · Mode v2 + per-mode hotkeys · templates + transforms ·
                        mode import/export · settings v3
Release D (2.x)         STT engine seam + privacy gate (v4) · history (opt-in) · per-app modes
```

## 1. P0 — Critical: first-run success and trust (Release A + the Refine merge)

| #    | Item                                                                                                                                                               | Spec                     | User impact                                                                             | Eng / Design | Depends on        | Effort |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------ | --------------------------------------------------------------------------------------- | ------------ | ----------------- | ------ |
| P0-1 | **Land the in-flight Refine work** (AI profiles, Polish `⌥⇧P`, refine toggle, insert-outcome model, settings v2)                                                   | REFINE.md                | Unlocks every later item; fixes the provider-tab UX debt in one move                    | M / done     | —                 | M      |
| P0-2 | **Esc cancels recording** (transient global shortcut while recording; cheat-sheet row)                                                                             | 01 §P0-A, 02 UX-12       | Every accidental activation today has no "never mind"; competitors all have it          | S / S        | —                 | S      |
| P0-3 | **Copy sweep**: job-aware HUD labels ("Cleaning up…" vs "Rewriting…"), sentence-case notices, strip `x error:` prefixes, tray "Copy last dictation", hint rewrites | 09 §2, 02 UX-08/16/17/20 | The single cheapest credibility upgrade; errors start naming the fix                    | S / S        | —                 | S–M    |
| P0-4 | **Model download failure surfaced** (inline `⚠ Download failed — Retry` row, onboarding + Models)                                                                  | 09 §3.2, 02 UX-15        | Kills the worst silent dead end at the exact step that gates first success              | S / S        | —                 | S      |
| P0-5 | **No-model recovery callout** in settings when nothing is installed                                                                                                | 02 UX-01, 09 §5.8        | A skipped onboarding no longer strands a permanently broken app                         | S / S        | callout primitive | S      |
| P0-6 | **HUD success flash** (`inserted` status, ✓ + inserted text ≤ 1.5 s) + notice/error glyphs (ⓘ/⚠)                                                                   | 09 §3.4–3.5, 01 §P1-D    | Confirmation where eyes already are; severity survives color-blindness; perceived speed | M / S        | IPC mirror        | M      |
| P0-7 | **Quick-win hint batch**: rewrite-needs-AI hint, dictionary ghost-row empty state, Modes subtitle, tap-latch copy, EN-only language disable, Uses-AI fallback hint | 02 top-10, 05 §3–4       | Closes most first-run confusion for ~zero engineering                                   | S / S        | —                 | S      |

P0 exit criterion: a first-time user who skips everything still ends up in an app that tells
them what is missing, and a user who dictates daily sees accurate, severity-legible feedback.

## 2. P1 — High: the v2 core (Releases B and C)

| #    | Item                                                                                                                                                                                                             | Spec                        | User impact                                                                                           | Eng / Design | Depends on                                 | Effort |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- | ----------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------ | ------ |
| P1-1 | **Onboarding rebuild**: consent download on Welcome (parallel, off critical path), in-window real try-it, raw→cleaned diff, "You're set" teaching exactly three things, honest skip summary, re-run from General | 04                          | First success in ~40–55 s active; the cleanup value is demonstrated, not claimed; no dead ends        | M / M        | P0-4                                       | M      |
| P1-2 | **Discoverability top-5**: hotkey cheat-sheet (General), "Add correction" on Last result, "Browse templates…" entry, hint-line pass, tip system core (`tip.modes`, `tip.ai`)                                     | 05 §8.3                     | Every major feature gains a durable teaching surface without nags                                     | M / M        | tips fields (v3); P1-6 for templates entry | M      |
| P1-3 | **Design tokens + dark-mode override** (Appearance: System/Light/Dark) + the verified AA contrast fixes                                                                                                          | 09 §1, §4.5                 | Real dark-mode control; WCAG AA pass; every later screen inherits the system                          | M / S        | —                                          | M      |
| P1-4 | **Accessibility batch**: HotkeyRecorder un-trap + announce, sidebar tablist, Cmd+W/Esc close, Toggle required label, ModesTab radiogroup + Active pill + legend, HUD `aria-live`, reduced motion                 | 09 §4, 02 UX-03/05/14/30/31 | Keyboard and screen-reader users stop hitting traps; the dual-gesture row becomes legible to everyone | M / M        | —                                          | M–L    |
| P1-5 | **IA reshuffle to seven pages** (Dictation / Modes / Models / Output / Dictionary / General / About; toggle re-homed to "After transcribing"; profiles re-homed to Models)                                       | 03                          | Each page answers one question; setup and recovery paths become findable                              | M / M        | P0s, P1-3 (tokens first)                   | M–L    |
| P1-6 | **Mode templates + the `transforms` split** (`SAFETY_RULES` / `DEFAULT_BEHAVIOR`, 9 production prompts, gallery sheet)                                                                                           | 06 §1–2                     | Custom modes go from blank-textarea to one-click personas; translation becomes possible _and_ safe    | M / M        | — (rides v3)                               | M      |
| P1-7 | **Mode v2 overrides + per-mode hotkeys** (aiProfileId/sttModelId/language/hotkey, inherit-by-default; one-shot hotkeys, ≤ 5; resolution + dangling fallbacks; `Listening — <Mode>`)                              | 07                          | The brief's "profiles" delivered with zero new concepts; power users skip the tray entirely           | M–L / M      | P0-1 (v2), settings v3                     | L      |
| P1-8 | **Mode import/export + drag-drop** (versioned JSON, validation, `docs/modes/` + Discussions convention)                                                                                                          | 06 §4                       | Community sharing with no marketplace and no service                                                  | M / S        | P1-6                                       | M      |
| P1-9 | **LLM provider presets** (`LLM_PRESETS` registry + `presetId`; Anthropic/Gemini/Groq/OpenRouter/Mistral/LM Studio/llama.cpp prefills)                                                                            | 08 §1                       | "Any model" stops meaning "go find the base URL yourself"                                             | S / S        | P0-1                                       | S      |

## 3. P2 — Medium: depth and power (Release D)

| #    | Item                                                                                                                                                                                            | Spec                | User impact                                                                                                 | Eng / Design | Depends on              | Effort |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------- | ------------ | ----------------------- | ------ |
| P2-1 | **STT engine seam + generic OpenAI-audio client + privacy gate** (SttEngine trait, `SttProfile`, engine-qualified ids (v4), audio-leaves-Mac consent dialog, tray/HUD locality, docs rewording) | 08 §2–6             | Faster-Whisper/whisper-server/OpenAI/Groq STT in one move — with consent that matches the product's promise | L / M        | P1-5 (Models page), v4  | L      |
| P2-2 | **Mode Preview** (`test_mode` against the active profile; rules-fallback offline)                                                                                                               | 06 §6               | Prompt authors see what a mode does before trusting it                                                      | S / S        | P1-6                    | S      |
| P2-3 | **Remaining tips** (`tip.latch`, `tip.polish`, `tip.accuracy` + `hudTip` on the success payload)                                                                                                | 05 §2               | The last zero-affordance features gain a moment                                                             | S / S        | P0-6, P1-2              | S      |
| P2-4 | **Opt-in local history + reprocessing** (default off, local file, search, re-run through a different mode/profile)                                                                              | 01 §P1-A/B, ROADMAP | The most-cited power-user retention feature in the category; default-off preserves the privacy story        | L / M        | P1-7 (resolution reuse) | L      |
| P2-5 | **Dictionary import/export (CSV)**                                                                                                                                                              | 01 §P2-C, ROADMAP   | Unoccupied niche across all competitors; serves teams and reinstalls                                        | S / S        | —                       | S      |
| P2-6 | **Per-app modes** (bundleId → modeId rules; reuses 07's one-shot path)                                                                                                                          | ROADMAP, 07 §9      | The most-cited Superwhisper gap; the tool disappears into the workflow                                      | L / M        | P1-7                    | L      |
| P2-7 | **Perceived-latency work**: streaming-feel transcription progress, audio cues, menu-bar recording indicator                                                                                     | ROADMAP, 01 §P1-D   | Perceived speed is the top complaint axis in the category                                                   | M / S        | —                       | M      |
| P2-8 | **Ollama auto-detect quick-add** (ping localhost:11434, prefilled profile)                                                                                                                      | 01 §P2-D            | Removes the last manual steps from the biggest single-step upgrade                                          | S / S        | P0-1                    | S      |

## 4. P3 — Future

| #    | Item                                                                    | Spec     | Why it waits                                                                 | Effort |
| ---- | ----------------------------------------------------------------------- | -------- | ---------------------------------------------------------------------------- | ------ |
| P3-1 | Bespoke STT engines: Deepgram, AssemblyAI                               | 08 §7 P3 | The generic client (P2-1) covers four engines first; these are one-impl-each | M each |
| P3-2 | Privacy-demonstration onboarding screen (live zero-connections proof)   | 01 §P3-A | High trust value, niche reach; needs network-stat plumbing                   | M      |
| P3-3 | Non-native-speaker onboarding path (locale-aware copy, Polish surfaced) | 01 §P3-B | Valuable segment; rides the next onboarding iteration                        | S      |
| P3-4 | API keys in the macOS Keychain (key-reference field in profiles)        | ROADMAP  | Orthogonal to UX; schema slot already reserved in REFINE.md                  | M      |
| P3-5 | Command mode + spoken punctuation/casing                                | ROADMAP  | New grammar surface; design after per-app modes prove the resolution layer   | L      |
| P3-6 | Settings search                                                         | 03 §4    | Below the page-count threshold until history + per-app rules land            | S      |
| P3-7 | `{{app}}` variable                                                      | 06 §3    | Waits on per-app detection (P2-6)                                            | S      |
| P3-8 | Windows/Linux, alternative local engines (Parakeet), CoreML encoder     | ROADMAP  | Platform/engine work, not UX; the 08 trait is the seam they plug into        | L      |

## 5. Rejected (and why)

- **Wake word ("Hey Flow").** Requires an always-listening microphone — architecturally
  incompatible with "records only while you hold the hotkey" (04 §4.2 copy, PRIVACY.md). Not a
  backlog item; a posture.
- **A second switchable "profile/persona" concept.** Rejected in 03 D1 / 07 §1; personas ship
  as mode templates.
- **Telemetry-driven onboarding funnels, badges/red-dot nudges, modal tours.** 00 §8.1, 05 §7.
- **Marketplace / hosted template or engine catalogs / cloud sync.** 00 §8.8; sharing is files
  (06 §4), engines are compiled-in (08 §7).
- **Silent local→cloud fallback of any kind.** 08 §4.3: automatic fallback may only move toward
  _less_ data leaving the machine.

## 6. Dependency notes

- The tips system (P1-2, P2-3) needs its settings fields and the success-flash event; both ride
  **v3** with Mode v2 — ship the schema once.
- The IA reshuffle (P1-5) is the only structural PR; everything in P0 plus tokens/copy/states
  lands before it by design (09 §7), so review risk stays low.
- History (P2-4) deliberately follows Mode v2: reprocessing reuses 07's resolution and 06's
  prompts, so building it earlier would build it twice.
- Cloud STT (P2-1) must not ship before the IA's Models page exists — the consent UX and the
  locality badges are designed against that surface, and the README/PRIVACY rewording (08 §3.4)
  is a hard gate for the release that includes it.
