# OpenFlow v2 — product design package

Status: complete design set. Written 2026-06-11 against `main` (cd63494) plus the in-flight
Refine work (`docs/REFINE.md`). Eleven documents: a factual baseline, two research documents,
seven implementation-ready designs, and a prioritized roadmap. Mission: make OpenFlow the most
intuitive, flexible tool in its category — a first-time user understands every major feature
without documentation; power users keep full control over providers, prompts, and workflows.

## How to read this

| Doc                                                             | What it is                                                                                         |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| [00 — Current state](00-current-state.md)                       | Factual inventory of the shipping app + in-flight work; canonical vocabulary; hard constraints     |
| [01 — Competitive analysis](01-competitive-analysis.md)         | Ten products; comparison matrix; gap analysis; prioritized opportunities                           |
| [02 — UX audit](02-ux-audit.md)                                 | 34 findings (4 critical), every one citing `file:line` evidence; top-10 quick wins                 |
| [03 — Information architecture](03-information-architecture.md) | The keystone: one switchable concept (D1), seven concrete pages (D2), sitemap, 12 user flows       |
| [04 — Onboarding](04-onboarding.md)                             | First real dictation in < 60 s active; screen-by-screen spec with exact copy and failure paths     |
| [05 — Discoverability](05-discoverability.md)                   | 18-capability education map; 6-tip system (local-state-triggered); empty states; cheat-sheet       |
| [06 — Custom instructions](06-custom-instructions.md)           | Prompt layering made visible; `transforms` split; 9 ship-ready templates; variables; import/export |
| [07 — Profiles](07-profiles.md)                                 | The brief's "profiles" as Mode v2: inherit-by-default overrides, one-shot hotkeys, resolution      |
| [08 — Provider extensibility](08-provider-extensibility.md)     | LLM presets over plugins; a real `SttEngine` trait; the audio-leaves-the-Mac privacy gate          |
| [09 — UX polish](09-ux-polish.md)                               | Token system + dark-mode override; copy sweep; state matrix; a11y fixes; before/after screens      |
| [10 — Roadmap](10-roadmap.md)                                   | P0–P3 with impact, complexity, dependencies, effort; release grouping; rejected items              |

Read 00 → 03 → 10 for the shape; the rest are reference designs for whoever builds each piece.

## Executive summary

1. **The core loop is good; the debt is discoverability and naming** (02). OpenFlow's best
   features — tap-latch hands-free, the rewrite hotkey, Copy Last Result, the never-lose-text
   fallback — have zero teaching surface. Most of P0 is copy, hints, and states, not engineering.
2. **One switchable concept, not two** (03 D1, 07). The brief's persona "profiles" ship as
   **Mode v2**: modes gain inherit-by-default overrides (AI profile, speech model, language,
   hotkey) and persona-tagged templates. This matches Superwhisper — the category's
   best-regarded customization model — and avoids a two-axis active-state matrix.
3. **Concrete pages beat abstract categories** (03 D2). Input/Processing/Models/Output/
   Personalization map onto seven findable pages: Dictation, Modes, Models, Output, Dictionary,
   General, About.
4. **Onboarding's job is one real success** (04). Move the model download off the critical path
   (consent on the Welcome screen, runs in parallel), make "Try it" a genuine in-window
   dictation, and show the raw→cleaned diff — demonstrate the value, never claim it.
5. **Extensibility is asymmetric** (08). LLM side: presets over plugins — every provider speaks
   OpenAI-compatible; ship prefills, not code paths. STT side: a real engine trait, because
   cloud STT uploads _audio_ and therefore needs a first-class consent gate, a louder badge,
   and the rule that automatic fallback may only move toward _less_ data leaving the machine.
6. **Privacy is the widening lead** (01). Wispr Flow's screenshot capture is now a mainstream
   complaint; OpenFlow's architecture-level, firewall-verifiable privacy is the wedge. The
   design makes it tactile: locality badges everywhere, consent dialogs that name what is sent,
   and a no-telemetry tip system computed from local state only.

## Decision register

| Decision                                                                                              | Where        |
| ----------------------------------------------------------------------------------------------------- | ------------ |
| Modes are the one switchable bundle; personas = mode templates                                        | 03 D1, 07 §1 |
| Seven concrete settings pages; no literal category tabs                                               | 03 D2        |
| `SHARED_RULES` splits into invariant `SAFETY_RULES` + optional `DEFAULT_BEHAVIOR` (`transforms` flag) | 06 §1        |
| Templates are a static in-binary registry; copies never auto-update                                   | 06 §2, §8    |
| Mode hotkeys are one-shot (dictate in that mode; active mode unchanged), capped at 5                  | 07 §4        |
| Override resolution at job start; dangling refs fall back to global + named notice, never fail        | 07 §3        |
| LLM providers = presets (data) over one client; `LlmProviderKind` gains no variants                   | 08 §1        |
| STT engines = `SttEngine` trait; generic OpenAI-audio client covers 4 engines first                   | 08 §2        |
| Cloud STT requires per-engine consent naming what is sent; fallback only cloud→local                  | 08 §3–4      |
| Tips trigger from local state only; ≤ 1/day; everything restated in durable surfaces                  | 05 §2, §7    |
| No second profile concept, no wake word, no marketplace, no telemetry, no silent cloud                | 10 §5        |

## What v2 should look like if launched today

**The 60-second first run.** You launch OpenFlow, read three privacy bullets, and click
"Download Base (English)" — the one explicit network action, running in the background while
you grant the microphone and decide about Accessibility (clipboard-only is presented as a
working outcome, not a failure). On the "Try it" screen you dictate into a real text field in
the window and watch your "um so yesterday I I shipped…" become clean text — raw and cleaned
side by side. The final screen points at the menu bar and teaches exactly three things: tap to
latch hands-free, `⌥⇧Space` rewrites a selection, modes live in the menu bar. Typical active
time: ~55 seconds.

**Daily use.** Hold `⌥Space` anywhere; the HUD says "Listening — Notes" so you always know
which mode will write. Release, and a ✓ flash shows the inserted text. Esc cancels. If a paste
goes wrong, the HUD says where your text is, and the menu bar's "Copy last dictation" always
has it. Select text and tap `⌥⇧P` to fix grammar silently, or hold `⌥⇧Space` and say "make it
shorter."

**Customization that explains itself.** The Modes page opens with "Output styles — how your
dictation is written." A template gallery (Email, Commit message, Meeting notes, Translation,
Slack, Academic, Support reply, Study notes, Social post) turns a blank textarea into one-click
personas; the editor states what OpenFlow appends automatically, previews any prompt against a
sample, and an Advanced section adds per-mode AI profile / speech model / language / hotkey —
every row "Inherit — currently …" so the bundle never surprises. Modes export as small JSON
files; sharing is files and a repo gallery, not a marketplace.

**Models without mystery.** One Models page shows what runs your speech (on-device whisper
models, with sizes and a download/Retry story) and your AI (profiles with `local`/`cloud`
badges derived from the URL, preset prefills for OpenAI/Anthropic/Gemini/Groq/OpenRouter/
Mistral/LM Studio/llama.cpp, one Test button each). "No AI" is a first-class choice, and the
product is fully functional — rules-based cleanup, offline — without ever visiting this page.

**What v2 deliberately does not include.** Opt-in history and reprocessing, per-app mode
rules, and cloud STT engines are designed (01, 07 §9, 08) but staged after 2.0 — each depends
on v2's resolution layer or its Models surface, and cloud STT additionally gates on rewording
the privacy claims it touches. No wake word, ever, on the current posture: the microphone is
live only while you hold the key.

**Positioning in one sentence.** The dictation tool that is free, unlimited, and provably
private — and now as effortless on day one as the subscription apps, with more control than
any of them.

## Success criteria (no telemetry — observable by construction and by hand)

- First real dictation in < 60 s of active time on a typical connection (04 §2).
- Every capability in 05 §1's table has at least one durable teaching surface.
- Zero network connections with no profile configured, verifiable with Little Snitch (PRD).
- `pnpm lint && pnpm format:check && pnpm typecheck && pnpm -r test` and
  `cargo fmt --check && cargo clippy -- -D warnings && cargo test` stay green throughout.
