# 01 — Competitive analysis: UX benchmarking and opportunity map

Status: research document. Written 2026-06-11. Web-verified where possible; uncertain claims
marked (unverified). Treats 00-current-state.md as the factual baseline for OpenFlow.
Uses OpenFlow vocabulary from §7 of that document (Mode, AI profile, Polish selection, etc.).

---

## 1. Method and scope

Ten products were analyzed: five direct competitors (Wispr Flow, Superwhisper, Refine (refine.sh
/ getrefine.app), Raycast AI, MacWhisper) that compete on dictation or AI text handling on
macOS; and five transferable-pattern products (ChatGPT Desktop, Claude Desktop, Cursor,
TypingMind, Open WebUI) that are not dictation tools but demonstrate strong patterns in
onboarding, provider management, custom instructions, and command surfaces worth adapting to
OpenFlow. The five transferable-pattern products are analyzed for those UX patterns only, not
as dictation competitors. Primary sources: live websites, product changelogs, and web
search results as of June 2026. Where a feature could not be confirmed, it is marked (unverified).

---

## 2. Per-product UX analysis

### 2.1 Wispr Flow

**What it is:** Cloud-only, AI-first dictation app for macOS/Windows/iOS/Android. Every
keystroke of audio goes to cloud servers; context awareness via periodic screenshots.

**UX strengths**

- Context-aware formatting is the headline differentiator: output tone shifts automatically
  based on the active app (Slack → casual, email → formal). No explicit mode switching needed.
- Cross-device sync of custom dictionary, snippets, and style preferences. Start a draft on
  Mac, finish on iPhone — same vocabulary follows.
- "Hey Flow" wake word enables hands-free activation without a hotkey press.
- 14-day trial requires no credit card, removing the signup barrier.
- Command Mode (Pro) enables inline AI text editing across any app.
- 100+ language support with code-switching (mixed-language utterances) auto-detected.

**UX weaknesses**

- All audio and screenshots go to cloud servers. Viral Reddit threads and independent reviewers
  document screenshot capture as a dealbreaker for regulated industries and privacy-conscious
  users. No on-device mode exists.
- Free tier caps at 2,000 words/week — roughly 8 minutes of dictation per day. The cap is the
  single most-mocked aspect of the product in reviews.
- ~800 MB RAM, ~8% CPU reported on 2021 MacBook Pro idle. Electron-based.
- Trustpilot score is 2.7/5 despite a high App Store rating — user-reported quality degradation
  after trial ends, and audio routed through OpenAI and Meta servers per independent review.
- No local STT option; product fails completely with network blocked.
- $15/month with no lifetime option — high total cost of ownership vs. alternatives.

**Onboarding approach:** Standard permission flow, 14-day trial, no credit card. The value
proposition (context-aware formatting) is shown immediately; technical setup is hidden.

**Discoverability:** Context-switching behavior surfaces itself during normal use. Users notice
the tone difference in Slack vs. email before they explicitly learn about it.

**Customization model:** Custom dictionary, snippets, style preferences sync across devices.
Command Mode adds inline AI editing. No explicit "modes" concept; context-awareness is
automatic and opaque.

**Patterns worth stealing:**

- Automatic per-app tone adaptation (maps to OpenFlow's per-app mode switching, on ROADMAP).
- Wake word as an alternative activation path.
- The "no credit card trial" reduces commitment friction for new installs.

---

### 2.2 Superwhisper

**What it is:** AI dictation app for macOS/Windows/iOS. Local-first with optional cloud STT and
LLM. Bootstrapped, privacy-award-winning product.

**UX strengths**

- **Modes as bundles** is the best-executed customization model in this category. Each mode
  packs: an STT model, an optional LLM post-processing prompt, a dedicated hotkey, and an
  auto-activation rule (based on active app or website). Switch modes = switch the entire
  dictation pipeline, not just a prompt.
- Auto-activation per app is fully implemented: "use Email mode in Mail, Slack mode in Slack"
  without touching a hotkey or menu.
- History with full-text search, segmented playback, and reprocessing against a different mode
  (v2.8+). History is opt-in and local.
- Model library: users browse and compare STT models before selecting (v2.8+).
- BYOK for OpenAI, Anthropic, Google, Groq, Meta, Mistral, Grok — one configuration surface
  covers multiple providers. Pro users get per-mode model selection.
- Nvidia Parakeet support (25 languages) alongside Whisper models; local STT on Apple Silicon
  can reach 300x real-time speed.
- Onboarding toasts guide new users contextually after setup (v2.13.0+).
- 4.9/5 Product Hunt; Privacy Award Winter 2025. Bootstrapped (no VC pressure to monetize data).

**UX weaknesses**

- $249.99 lifetime or $8.49/month. Paid wall blocks full mode system — free tier is capped to 3
  modes and small local models only.
- No selected-text rewrite via voice (as a first-class feature). Dictation into an active text
  field is the primary flow.
- Windows and iOS versions are less mature than macOS.
- "Modes" vocabulary overloaded: Superwhisper's mode = full pipeline bundle; OpenFlow's mode =
  output style only. Risk of confusion when users migrate.

**Onboarding approach:** Rebuilt flow (v1.44.0); onboarding toasts contextually appear after
first use. Model selection happens early. The modes concept is introduced progressively.

**Discoverability:** Auto-activation makes modes discoverable through use — the app switches
mode and users notice the formatting change without reading documentation.

**Customization model:** Modes bundle STT model + LLM model + prompt + output behavior + hotkey

- auto-activation rule. Per-mode hotkeys let power users skip the menu entirely.

**Patterns worth stealing:**

- The full modes-as-pipeline-bundle concept (not just prompt selection).
- Per-mode dedicated hotkeys.
- History reprocessing: apply a different mode to an existing transcript without re-dictating.
- Model library UX for comparing STT options before download.
- Onboarding toasts that appear contextually after first successful dictation.

---

### 2.3 Refine (refine.sh + getrefine.app)

**What it is:** Two products share this name. refine.sh is a local-first grammar/style checker
(not dictation). getrefine.app is a free, MIT-licensed open-source text rewriter triggered by
hotkey on selected text. Both are privacy-first and BYO-key.

**UX strengths (refine.sh)**

- Custom prompts and style guides: users define exactly how checking should behave. Closest
  thing to OpenFlow's per-mode prompts, but for passive proofreading.
- Real-time and on-demand modes. Inline translation when writing in a non-English phrase.
- Explanation for each suggestion ("why was this changed"), which increases trust and aids
  learning for non-native speakers.
- 7-day free trial; one-time purchase model available (price unverified).

**UX strengths (getrefine.app)**

- `Cmd+Shift+R` on any selected text → rewrite, fix, translate, or transform. Same core flow
  as OpenFlow's Rewrite selection and Polish selection features.
- Chainable modes (flows): run mode A then mode B in sequence with one hotkey.
- MIT-licensed. No account required. Ollama + OpenAI + Anthropic supported.
- Free to use.

**UX weaknesses**

- refine.sh: grammar-checker positioning, not dictation; different user intent.
- getrefine.app: no dictation, no STT. Only useful when you already have text to operate on.
- Chainable flows add power but also conceptual complexity for new users.
- Both: less polished onboarding than Superwhisper or Wispr Flow; more power-user oriented.

**Onboarding approach:** Refine.sh has a 7-day trial with an emphasis on the privacy claim as
the first screen. getrefine.app is GitHub-hosted; onboarding is docs-only.

**Customization model:** Custom prompts per mode; chainable flows. No AI profile abstraction —
provider is configured once globally.

**Patterns worth stealing:**

- Chainable mode flows (run Standard → Email in sequence) — maps to OpenFlow's pipeline idea.
- Explanation-per-change in the output, especially valuable for non-native-speaker user segment.
- The refine.sh onboarding pattern: lead with the privacy claim as a visual, concrete screen.

---

### 2.4 Raycast AI (Dictation, v2 beta)

**What it is:** macOS productivity launcher adding AI dictation system-wide in v2 public beta
(May 2026). Hold or tap hotkey → speaks → pastes clean formatted text. Per-app Auto Styling.

**UX strengths**

- Auto Styling per app is analogous to Wispr Flow's context awareness: email tone in Mail,
  quick message tone in Slack, automatically. Configurable with custom instructions per app.
- Dictation history surfaced as a named command, not buried in a settings tab. "Copy Last
  Transcription" is an explicit command, not a tray menu item.
- Dictation Pill: a floating indicator during recording — a well-executed visual metaphor that
  stays in flow without covering the active app (similar intent to OpenFlow's HUD).
- The launcher context means Raycast AI dictation is discovered as part of a broader tool
  users already trust. No separate install needed.
- Profiles (role + tools + communication style) and Memory (built over time from conversations)
  address persistent personalization across all AI features in one place.
- Free during beta.

**UX weaknesses**

- Dictation is a feature inside Raycast, not the product. Users who only want dictation pay for
  the full Raycast subscription.
- Requires macOS Tahoe for v2 (as of beta); Sequoia users must wait or run both v1 and v2.
- Cloud-based transcription; no local STT option announced for dictation.
- Custom vocabulary, instructions, and styles exist but their UI and discoverability are less
  mature than Superwhisper's modes system (unverified — in beta at time of writing).
- No selected-text rewrite via voice (unverified).

**Onboarding approach:** Dictation is discovered through Raycast's command palette during normal
use. No separate onboarding for the dictation feature; it inherits Raycast's mature onboarding.

**Customization model:** Per-app instructions; custom vocabulary; hold or tap hotkey behavior.
Profiles and Memory are global across all Raycast AI features.

**Patterns worth stealing:**

- Surfacing "Copy Last Transcription" as an explicit named command users can trigger and search
  for, not just a menu item they have to find.
- The Dictation Pill as a visual pattern for non-intrusive recording feedback.
- Profiles + Memory as a combined personalization surface — relevant for OpenFlow's AI profile
  concept plus any future user preference persistence.

---

### 2.5 MacWhisper

**What it is:** macOS file transcription app by indie developer Jordi Bruin. Drag-and-drop audio
or video files → local Whisper or optional cloud → transcript. System-wide dictation added
as a secondary feature.

**UX strengths**

- The transcription use case is the clearest product focus in the category. Drag a file,
  get a transcript. No friction, no modes, no configuration required for basic use.
- One-time payment (~$29 on Gumroad). Best value in the category for a local, private tool.
- Model library browsing: Tiny through Large-v3 Turbo plus Parakeet v2/v3. Users can compare
  model sizes, speed, and language support before selecting.
- Speaker identification and SRT/VTT/PDF/HTML export (Pro). Niche but differentiating for
  meeting transcription use case.
- 4.8/5 on Product Hunt with 1,900+ ratings. Privacy-first messaging is prominent.
- Optional cloud (Deepgram, Groq, ElevenLabs) is clearly opt-in and visually distinct.

**UX weaknesses**

- System-wide dictation is a secondary feature bolted onto a file-transcription tool. The UX
  for hotkey dictation is noticeably less polished than Superwhisper or Wispr Flow.
- No modes, no AI post-processing prompts, no selected-text rewrite.
- No LLM integration for cleanup — transcript output only.
- Onboarding is minimal; documentation assumes technical comfort.
- No HUD-equivalent during recording; minimal visual feedback.

**Onboarding approach:** First-launch shows a model download prompt. Core experience (drag file
→ transcribe) is discoverable without documentation.

**Customization model:** Model selection, speaker identification settings, export format
preferences. No prompt customization; transcription only.

**Patterns worth stealing:**

- One-time payment positioning to reinforce the "no subscription, open, free" story.
- The model selection UX: size, speed, language summary in one row before download.
- Privacy messaging as a headline on the product page, not buried in fine print.

---

### 2.6 ChatGPT Desktop (macOS) — transferable patterns only

**What it is:** OpenAI's macOS desktop app. Conversational AI, voice, computer use, coding
agent. Analyzed for UX patterns, not as a dictation competitor.

**Patterns worth stealing for OpenFlow:**

- Global `⌥Space` hotkey for quick AI access (same default as OpenFlow — validates the
  hotkey convention). The companion window "stays in front of all windows" pattern maps to
  OpenFlow's HUD-always-present approach.
- Per-context AI customization (custom instructions configurable per chat / project) shows
  how to make prompt-level personalization feel accessible to non-technical users.
- The "Slash commands" pattern for quickly switching behavior in-flow — relevant to OpenFlow's
  potential command mode (on ROADMAP).
- Notifications for background task completion maps to OpenFlow's pattern of HUD notice + tray
  recovery; ChatGPT's approach is more visible to users working across many windows.

---

### 2.7 Claude Desktop — transferable patterns only

**What it is:** Anthropic's macOS/Windows/iOS desktop app. Chat, agentic (Cowork), code agent.
Analyzed for UX patterns, not as a dictation competitor.

**Patterns worth stealing for OpenFlow:**

- One-click MCP connector marketplace reduced a two-minute config-file install to a single
  click. This is the gold standard for BYO-provider UX. OpenFlow's AI profile setup (manual
  base-URL + key entry) is the equivalent pain point to solve.
- "Local vs. cloud" badge derived from the connection type (localhost → local; remote → cloud)
  is exactly the pattern OpenFlow already implements for AI profiles — validation that the
  approach is correct and recognizable to users.
- Three modes (Chat / Cowork / Code) in a single shell with a sidebar switcher shows how to
  surface multiple job types without a separate app. Relevant if OpenFlow adds command mode or
  transcription-only mode alongside dictation.
- Project-scoped custom instructions (custom system prompts per project) maps directly to
  OpenFlow's per-mode prompt customization.

---

### 2.8 Cursor — transferable patterns only

**What it is:** AI-native code editor (VS Code fork). Analyzed for custom instructions and
onboarding UX patterns, not as a dictation competitor.

**Patterns worth stealing for OpenFlow:**

- Import existing settings on first launch. Cursor detects VS Code and pulls extensions,
  settings.json, and keybindings. OpenFlow analog: detect an existing settings.json on first
  launch and offer to continue where the user left off — useful for re-installs after a
  fresh macOS setup.
- Project-level + personal-level rules hierarchy. Users can set personal conventions and
  project rules that stack without conflicting. OpenFlow analog: a global default mode stacked
  with per-app mode overrides.
- `.cursor/rules` files checked into git — rules are portable and versionable. OpenFlow's
  profile files already follow this pattern (file-backed, hand-droppable). Reinforce this in
  documentation and the "Show in Finder" affordance.
- Agent mode progress visibility: Cursor shows which files it read and why. OpenFlow's HUD
  only shows pipeline stage label. Users want to know "what is it doing right now" when
  latency exceeds ~1 second.

---

### 2.9 TypingMind — transferable patterns only

**What it is:** BYO-key universal LLM frontend. Multiple providers, custom agents, prompt
library, one-time or subscription pricing. Analyzed for provider management and custom
instructions UX, not as a dictation competitor.

**Patterns worth stealing for OpenFlow:**

- **Provider management as a first-class UI surface.** TypingMind shows all configured
  providers, lets users test them with one click, and selects the active one per conversation.
  OpenFlow's AI profile tab is close to this but lacks the "active per context" layer.
- BYOK model shown at sign-up as the primary pricing story — users understand immediately they
  pay the provider, not the app, for LLM calls. OpenFlow's cloud-is-opt-in story is the same;
  it deserves equally prominent framing in onboarding.
- Per-agent custom system prompts (Characters) with a named, icon-ed UI make custom prompts
  feel like distinct personas rather than raw text boxes. Relevant to how OpenFlow surfaces
  mode prompts.
- One-time $39 purchase model proves users will pay once for an AI tool if the value is clear.
  Relevant as OpenFlow considers any future paid tier.

---

### 2.10 Open WebUI — transferable patterns only

**What it is:** Self-hosted, open-source web UI for Ollama and OpenAI-compatible backends.
Most popular local LLM frontend (50K+ GitHub stars). Analyzed for local LLM management and
onboarding patterns, not as a dictation competitor.

**Patterns worth stealing for OpenFlow:**

- Model management as a browseable catalogue. Open WebUI shows all available Ollama models
  with size, speed, and capability notes in a table. Users pull models directly from the UI
  without touching a terminal. OpenFlow's model manager already does this for STT; the pattern
  should extend to AI profiles (show what model is loaded, VRAM usage if Ollama, last-used).
- "Two-minute Docker install → first prompt" onboarding shows that local, technical software
  can still feel approachable. The mechanism: a single command with immediate visual payoff.
- System-prompt templates (Modelfiles / custom characters) are reusable named objects. Maps to
  OpenFlow's mode-as-named-prompt. The key UX lesson: give custom instructions a name and an
  icon so they feel like things, not free-form text boxes.
- Optional cloud backends shown alongside local ones in one unified list, with a clear
  "local / remote" label. Validates OpenFlow's existing local/cloud badge pattern.

---

## 3. Comparison matrix (direct competitors + OpenFlow)

Rows are capabilities. Columns are the five direct competitors plus OpenFlow.

| Capability                          | Wispr Flow              | Superwhisper                      | Refine (getrefine.app) | Raycast AI               | MacWhisper                      | **OpenFlow**                    |
| ----------------------------------- | ----------------------- | --------------------------------- | ---------------------- | ------------------------ | ------------------------------- | ------------------------------- |
| Hold-to-talk + tap-latch hands-free | ✓                       | ✓                                 | ✗                      | ✓                        | partial (system-wide secondary) | ✓                               |
| Toggle behavior (alt to hold)       | ✓                       | ✓                                 | ✗                      | ✓                        | ✗                               | ✓                               |
| Hands-free wake word                | ✓ (Hey Flow)            | ✗                                 | ✗                      | ✗                        | ✗                               | ✗                               |
| Modes / pipeline bundles            | partial (auto, opaque)  | ✓ full bundles                    | partial (prompts only) | partial (per-app styles) | ✗                               | partial (prompt + AI flag only) |
| Per-mode dedicated hotkey           | ✗                       | ✓                                 | ✗                      | ✗                        | ✗                               | ✗                               |
| Auto-activation per app             | ✓ (opaque)              | ✓ (explicit)                      | ✗                      | ✓ (beta)                 | ✗                               | ✗ (ROADMAP)                     |
| Custom prompts / modes              | ✓                       | ✓ unlimited (Pro)                 | ✓                      | ✓ (beta)                 | ✗                               | ✓ custom modes                  |
| LLM provider choice (BYOK)          | ✗ cloud-only            | ✓ multi-provider                  | ✓                      | ✗                        | partial (cloud opt-in)          | ✓ any OpenAI-compat             |
| STT provider choice                 | ✗                       | ✓ Whisper + Parakeet              | ✗                      | ✗ (cloud only)           | ✓ Whisper + Parakeet            | partial (Whisper only)          |
| Local STT                           | ✗                       | ✓                                 | ✗                      | ✗                        | ✓                               | ✓                               |
| Dictionary / vocabulary             | ✓ synced                | ✓                                 | ✗                      | ✓ (beta)                 | ✗                               | ✓                               |
| Selected-text rewrite (voice)       | ✗                       | ✗                                 | ✓ (text only)          | ✗ (unverified)           | ✗                               | ✓                               |
| Selected-text polish (no voice)     | ✗                       | ✗                                 | ✓                      | ✗                        | ✗                               | ✓ (in-flight)                   |
| History (searchable)                | ✗                       | ✓ (local, opt-in)                 | ✗                      | ✓ (local)                | ✓ (file-based)                  | ✗ (ROADMAP)                     |
| History reprocessing                | ✗                       | ✓                                 | ✗                      | ✗                        | ✗                               | ✗                               |
| Per-app behavior / mode override    | ✓ (automatic)           | ✓ (explicit)                      | ✗                      | ✓ (beta)                 | ✗                               | ✗ (ROADMAP)                     |
| Onboarding quality                  | good                    | good (rebuilt v1.44)              | minimal                | inherits Raycast         | minimal                         | adequate, incomplete            |
| Telemetry / cloud dependency        | ✗ required, screenshots | optional (cloud STT/LLM)          | ✗ local-first          | cloud STT                | optional                        | **✓ zero, verifiable**          |
| Price / openness                    | $15/mo, closed          | $8.49/mo or $249 lifetime, closed | free, MIT              | free (beta), closed      | ~$29 one-time, closed           | **free, MIT**                   |

---

## 4. Feature-gap analysis

### Where OpenFlow is behind

| Gap                                                   | Who does it best                  | Why it matters                                                                                                                                                                                       | Severity |
| ----------------------------------------------------- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| No per-app mode auto-activation                       | Superwhisper                      | Users dictate differently in Slack vs. Notes vs. code editors. Manual mode switching via tray is friction on every context switch.                                                                   | High     |
| No history                                            | Superwhisper, Raycast, MacWhisper | Users lose output if paste fails or they close the wrong window. "Copy Last Result" in the tray is one step, not searchable or replayable.                                                           | High     |
| No per-mode dedicated hotkey                          | Superwhisper                      | Power users switch modes by hotkey, not by opening the tray. Eliminates a 2–3 step flow.                                                                                                             | Medium   |
| Modes are prompt-only, not full pipeline bundles      | Superwhisper                      | Users cannot set "this mode uses Parakeet, that one uses large-v3" — one STT model for all modes.                                                                                                    | Medium   |
| No STT engine choice beyond Whisper                   | Superwhisper, MacWhisper          | Parakeet is materially faster for English on Apple Silicon. OpenFlow's single-engine lock is an architectural simplification that is now visible as a limitation.                                    | Medium   |
| Onboarding omits rewrite hotkey, modes, AI refinement | Superwhisper                      | Users complete setup and do not know three core features exist. Discovery requires reading docs.                                                                                                     | Medium   |
| No history reprocessing                               | Superwhisper                      | The ability to re-run an old transcript through a better or different mode is a power-user feature that distinguishes Superwhisper clearly.                                                          | Low      |
| No cancel via Esc                                     | Superwhisper, Wispr Flow          | `Esc` is the universal cancel convention. OpenFlow cancels only via new job or tray. Surprising behavior gap.                                                                                        | Medium   |
| No streaming-feel output                              | Wispr Flow, Superwhisper          | Text appears instantaneously after release in cloud tools. OpenFlow inserts all at once after transcription + refinement. Even faking streaming with a progress indicator reduces perceived latency. | Medium   |

### Where OpenFlow is ahead or differentiated

- **Verifiable privacy.** Audio cannot leave the machine by architecture, not policy. Network-
  blockable with Little Snitch. No screenshots. No telemetry. No account. Wispr Flow is the
  worst offender here (screenshot capture documented); Superwhisper's cloud proxy claims no
  retention but users cannot verify it.
- **Zero cost, unlimited.** No word cap, no subscription, no paid tier. Wispr Flow's 2,000
  word/week cap is its most-mocked feature; Superwhisper's free tier caps at 3 modes and
  small-only STT. OpenFlow is unlimited local by default.
- **Source-auditable.** MIT-licensed, readable code, no binary blobs. The privacy claim is
  verifiable, not a policy statement.
- **Selected-text voice rewrite + Polish selection.** No direct competitor offers both voice-
  instructed rewrite and a one-tap Polish hotkey as first-class features. Raycast AI v2 may
  be adding similar functionality (unverified, in beta).
- **AI-optional graceful degradation.** Modes degrade to rules-based cleanup if no LLM is
  configured. Product is fully useful offline and with zero cloud config.
- **Small footprint.** Tauri/Rust vs. Electron. OpenFlow does not idle at 800 MB RAM.

### Where every competitor is weak (open opportunity)

- **Explanation of AI changes.** No dictation tool shows the user what was changed from raw
  transcript to cleaned output, or why. Refine.sh does this for grammar checking; no one does
  it for dictation. A "show diff" affordance (raw vs. cleaned) would be unique.
- **Dictionary import/export.** All tools require manual entry of custom vocabulary. No tool
  supports importing a `.txt` wordlist or exporting to share across installs. OpenFlow's ROADMAP
  includes this; none of the direct competitors have shipped it.
- **Privacy-first onboarding as a differentiator.** Every privacy-first tool buries the
  privacy story in a marketing page. No tool uses onboarding itself to make the privacy
  architecture visible and tactile (e.g., a network monitor showing zero outbound connections
  during a dictation).
- **Non-native-speaker UX.** No tool explicitly addresses the non-native speaker segment in
  its onboarding or feature labeling. The Polish selection feature in OpenFlow is directly
  useful for this segment but is not surfaced in onboarding.
- **Reprocessing existing transcripts.** Superwhisper has history reprocessing but it applies
  the same mode to a past result. No tool enables "translate this existing dictation" or
  "re-run this through the Email mode" after the fact.

---

## 5. Prioritized opportunities

Ranked P0 (do first) to P3 (do later). Each item is self-contained for roadmap use.

---

### P0 — Critical: complete feature gaps users notice immediately

**P0-A: Esc to cancel**

- What: bind `Esc` as a global cancel shortcut that calls `cancel_dictation`.
- Evidence: `Esc` is the universal cancel on macOS. Every competitor supports it. Users who
  trigger a dictation accidentally have no obvious out without opening the tray.
- User impact: Eliminates surprise and frustration on accidental activations. Affects every
  user on every session.
- Cost: S (single shortcut binding in `shortcuts.rs`; `cancel_dictation` command already
  exists).

**P0-B: Extend onboarding to surface Rewrite selection, Modes, and AI refinement**

- What: Add a step 6 "More you can do" screen after the Try-it step. Show the rewrite hotkey,
  explain modes in one line, and mention AI refinement as optional. Also: add a "re-run setup"
  option in the About tab.
- Evidence: 00-current-state.md §4 explicitly notes these gaps. No competitor leaves their
  headline features invisible to new users. Superwhisper's onboarding toasts show a lighter
  approach that still works.
- User impact: First-time users currently discover rewrite and polish via tray exploration or
  docs. Moving this into onboarding increases activation of these features.
- Cost: S (one additional onboarding screen; a menu item in About tab).

**P0-C: "Show diff" affordance for Last Result**

- What: In the Last Result card (General tab), show the raw transcript alongside the cleaned
  output when they differ. Format as a simple before/after, not a technical diff.
- Evidence: No competitor does this; it is an open opportunity. Non-native speakers and users
  tuning mode prompts specifically want to see what the LLM changed. Dictionary + LLM changes
  are currently invisible.
- User impact: Builds trust in AI cleanup; helps users tune prompts; helps non-native speakers
  learn. Directly serves OpenFlow's stated user segment (non-native English speakers).
- Cost: S (data already exists: `rawTranscript` vs. final text; UI is one additional row in
  an existing card).

---

### P1 — High value: closes the largest competitive gaps

**P1-A: Searchable history (local, opt-in)**

- What: Persist the last N dictation results (default 50) to a local log file. Add a History
  tab in Settings. Support text search and one-click re-paste. Opt-in toggle in General tab
  with a privacy note ("stored only on this Mac").
- Evidence: Superwhisper and Raycast both have history; it is the most-requested feature in
  comparable tools. "Copy Last Result" is a partial mitigation but not searchable.
- User impact: Eliminates the frustration of losing output to paste failures or app switches.
  Unlocks reprocessing (P1-B). Directly addresses ROADMAP "opt-in history".
- Cost: M (new persistence layer; History tab UI; privacy copy).

**P1-B: History reprocessing (re-run a past transcript through a different mode)**

- What: In the History tab, each entry shows "re-run" → mode picker → re-sends transcript to
  the current AI profile with the selected mode's prompt.
- Evidence: Superwhisper's killer history feature. No other tool has it. Unlocked by P1-A.
- User impact: Users can fix a transcript they got in Literal mode by re-running through
  Standard mode. Eliminates re-dictation for missed mode selection.
- Cost: S (UI addition to P1-A; pipeline re-entry after the transcription stage already
  exists as a concept).

**P1-C: Per-mode dedicated hotkey**

- What: Add an optional "Hotkey" field to each mode (custom modes only in v1; all modes in
  v2). Pressing the mode's hotkey acts as dictation start/stop + mode override for that session.
- Evidence: Superwhisper implements this in v2.12.0. Power users cite it as the feature that
  makes mode-switching frictionless.
- User impact: Eliminates tray switching for users who work across multiple modes daily (e.g.,
  Literal for code, Email for messages).
- Cost: M (hotkey registration per mode; conflict detection; mode-scoped session concept).

**P1-D: Streaming-feel latency (progress during transcription)**

- What: Animate or show a character counter / "typing" effect in the HUD during the
  transcription stage. Does not require actual streaming from whisper; just makes the wait
  feel active.
- Evidence: Wispr Flow and Superwhisper produce near-instant text in cloud mode; OpenFlow's
  batch insert after silent wait feels slow by comparison even when total latency is similar.
  ROADMAP already lists "streaming-feel latency".
- User impact: Perceived latency is the top UX complaint in dictation tools. This change costs
  little and has outsized perceived improvement.
- Cost: S (HUD animation change; no pipeline change needed for the visual effect).

---

### P2 — Medium value: differentiation and power-user retention

**P2-A: Modes as partial pipeline bundles (add STT model field)**

- What: Add an optional `sttModelId` field to the mode schema. When set, that mode uses the
  specified STT model instead of the global default. Built-ins remain read-only; custom modes
  can set this.
- Evidence: Superwhisper's per-mode model selection is a genuine workflow upgrade: Literal mode
  can use tiny.en for fast code dictation; Email mode can use large-v3-turbo for highest
  accuracy. ROADMAP lists STT engine alternatives.
- User impact: Power users can optimize speed vs. accuracy per use case without changing the
  global model setting.
- Cost: M (mode schema change + IPC update; `stt.rs` must accept per-job model override).

**P2-B: Per-app mode auto-activation**

- What: Add a "Auto-activate in apps" field to each mode. Uses the active app's bundle ID (no
  additional permissions; macOS exposes front-most app to any process). Tray shows "(auto)"
  next to active mode when overridden.
- Evidence: Superwhisper and Wispr Flow both implement this. It is on OpenFlow's ROADMAP.
  Users who dictate across multiple apps cite it as the feature most likely to drive daily use
  over manual switching.
- User impact: Eliminates mode switching friction for users with distinct dictation contexts
  (code, email, notes). After setup, the tool disappears from the user's attention.
- Cost: L (bundle-ID detection; mode resolution order; tray label change; settings UI
  for mapping apps to modes).

**P2-C: Dictionary import/export**

- What: Add "Import from .txt" and "Export to .txt" buttons to the Dictionary tab. Format: one
  `from → to` pair per line, or `word` alone for vocabulary hints. Also: "Show in Finder"
  pointing to the dictionary section of settings.json.
- Evidence: No competitor has implemented this. It is a gap across the entire category and is
  on OpenFlow's ROADMAP. A shared wordlist (company names, medical terms, project names) is
  a recurring user need.
- User impact: Reduces setup time for new installs and enables sharing dictionaries within
  teams (relevant to privacy-required professional segment).
- Cost: S (file picker + parser; settings.json already holds the dictionary).

**P2-D: AI profile quick-add from tray**

- What: Add "Add AI profile…" as a tray item that opens the AI profile tab directly, pre-
  populated with provider detection (if Ollama is running locally, auto-fill localhost:11434).
- Evidence: TypingMind's provider management shows how a one-click test-and-save flow reduces
  the setup drop-off. OpenFlow's current flow requires: Settings → AI Provider tab → select
  provider → fill fields → test → save. Ollama auto-detection removes two manual steps.
- User impact: AI refinement is the largest single-step upgrade for existing users; reducing
  setup friction directly increases activation of the LLM features.
- Cost: S (tray menu item; detect Ollama via localhost:11434 ping; pre-fill UI).

---

### P3 — Lower priority: valuable but deferred

**P3-A: Privacy-demonstration screen in onboarding**

- What: After the "Try it" step, show a live network activity indicator during a dictation.
  Display zero outbound connections. Copy: "That dictation never left your Mac — by
  architecture, not policy."
- Evidence: This is an unoccupied niche across all competitors. It makes the privacy claim
  tactile rather than textual. Directly serves the privacy-required professional user segment.
- User impact: Increases trust and reduces security-team objections in regulated industries.
  High-value for the target segment; lower reach than P0-P2 changes.
- Cost: M (macOS network stats API or Little Snitch integration; onboarding screen).

**P3-B: Non-native-speaker onboarding path**

- What: At the Welcome step, add a language detector (from browser/OS locale). If non-English,
  show an additional tooltip: "OpenFlow will clean up grammar and phrasing in your chosen
  language — no need to think in English first." Surface Polish selection prominently.
- Evidence: OpenFlow's PRD identifies non-native speakers as a primary user segment; no
  competitor addresses them explicitly in onboarding. The Polish selection feature is directly
  valuable but invisible.
- User impact: Increases activation of Polish selection and AI cleanup features for this
  segment, which is large (Wispr Flow reports 60% non-English users industry-wide).
- Cost: S (locale detection + conditional copy in onboarding Welcome step).

**P3-C: Reprocessing transcript with alternative provider**

- What: In history (P1-A), add "Re-run with…" that lets the user select a different AI
  profile than the one active at time of dictation.
- Evidence: No competitor has this. Extends P1-B with provider flexibility.
- User impact: Useful for users with multiple AI profiles (e.g., local Ollama for normal use,
  OpenAI for higher-quality email mode).
- Cost: S (extension of P1-B; profile picker in history entry).

**P3-D: Audio cues**

- What: Optional start and stop tones, with a volume slider. Off by default.
- Evidence: On OpenFlow's ROADMAP. Both Superwhisper and Wispr Flow offer this. Users who
  dictate away from their screen (while walking, presenting) rely on audio cues to know the
  app is recording.
- User impact: Primarily useful for mobile-workflow users; lower priority for desktop-seated
  use which is the core scenario.
- Cost: S (a system sound or bundled tone; a settings toggle).

---

## 6. Key takeaways for v2

1. **Esc-to-cancel is a table-stakes gap.** Every competitor supports it. It is a single
   `shortcuts.rs` binding. Ship it in the next patch.

2. **Onboarding hides three core features.** Rewrite selection, modes, and AI refinement are
   invisible to users who click through setup. A single "More you can do" screen at step 6
   fixes this at near-zero cost.

3. **Superwhisper's modes-as-pipeline-bundles is the design target.** The current OpenFlow
   mode (prompt + AI flag) is a simplified subset. The full bundle (STT model + prompt + AI
   profile + hotkey + per-app activation) is achievable incrementally: per-mode hotkeys (P1-C)
   and per-mode STT model (P2-A) each add one field without a redesign.

4. **History + reprocessing is the highest-leverage power-user feature.** Superwhisper's
   searchable history and re-run-through-mode feature are repeatedly cited as the reason power
   users choose it over alternatives. OpenFlow has the architecture for this (ROADMAP); the
   question is implementation priority.

5. **The "show diff" affordance is a unique opportunity.** No competitor shows raw vs. cleaned
   transcript side by side. This is cheap to build (data already exists) and directly serves
   both the non-native-speaker and the prompt-tuning power user.

6. **Privacy is a genuine lead, not just a tagline.** Wispr Flow's screenshot capture and cloud
   dependency have become mainstream complaints, not just niche concerns. OpenFlow's verifiable,
   architecture-level privacy is a real wedge. The onboarding and settings UI should make this
   visible and tactile, not just stated.

7. **Streaming-feel latency matters more than raw latency.** Users perceive instant-insert (cloud
   tools) as faster than batch-insert (OpenFlow) even when the wall-clock time is similar.
   A cheap animation during the transcription stage closes most of the perception gap.

8. **The free tier word-cap is the most-mocked aspect of Wispr Flow.** OpenFlow's unlimited
   local is a concrete, quantifiable differentiator. Say so explicitly in the Settings/About
   tab and in onboarding.

9. **Dictionary import/export is an unoccupied niche.** No direct competitor has shipped it.
   It is a lightweight feature (S cost) that disproportionately serves the privacy-required
   professional and developer segments.

10. **AI profile setup is the activation bottleneck.** Users who complete onboarding without
    configuring an AI profile are less likely to return to do it later. Ollama auto-detection
    (P2-D) and clearer "optional but powerful" framing at the AI provider step reduce this
    drop-off.
