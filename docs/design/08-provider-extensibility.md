# 08 — Provider extensibility: LLM presets and a real STT engine seam

Status: design proposal. Written 2026-06-11 against `main` (cd63494) + the in-flight Refine
work (`docs/REFINE.md`, `profiles.rs`). Baseline facts and hard constraints: `00-current-state.md`
(especially §8). Page home and naming: `03-information-architecture.md` (the **Models** page —
Speech recognition + AI profiles). Bundling/switching and dangling-reference rules:
`07-profiles.md` (mode overrides). This document **extends** the file-backed `LlmProfile` design;
it never replaces it.

The brief asks Velata to support many providers on both sides of the pipeline: on the LLM side
OpenAI, Anthropic, Gemini, Groq, OpenRouter, Mistral, LM Studio, llama.cpp, Ollama, and custom
OpenAI-compatible endpoints; on the STT side whisper.cpp (today), Faster-Whisper/whisper-server,
the OpenAI Whisper API, Groq, Deepgram, and AssemblyAI. The two sides need very different
treatments and that asymmetry is the central design decision:

> **The LLM side is already extensible — extensibility there is data, not code.** The STT side is
> not, because cloud STT sends _audio_ off the machine, which is a different wire protocol _and_ a
> different privacy contract. So: LLM = a static **preset registry** over the one existing client;
> STT = a real **`SttEngine` trait** with a privacy gate in front of every cloud engine.

---

## 1. LLM side — presets over plugins

### 1.1 The decision and why

Velata already routes every provider — Ollama, OpenAI, Groq, OpenRouter, LM Studio,
llama.cpp — through **one** OpenAI-compatible `/v1/chat/completions` client (`llm.rs`; 00 §8,
"one LLM client for all providers"). That is a stated architecture decision and it stays. Adding
"providers" is therefore not a code problem; it is a _prefill_ problem. Users get base URLs and
auth styles wrong, so we ship the correct values as data.

The in-flight `LlmProviderKind` enum (`profiles.rs`) is `ollama | openaiCompatible` and **gains no
new variants**. Every provider in the brief speaks the same wire protocol on the same endpoint; the
only behavioral fork today is Ollama's _native_ model-listing (`GET /api/tags`), already special-cased
in `llm.rs::list_ollama_models`. Adding an `anthropic` or `gemini` variant would multiply code paths
— connect hints, request shaping, error mapping — for **zero behavioral difference**, because both
expose OpenAI-compatible chat-completions endpoints. The enum encodes _behavior_; a preset is
_display + prefill_. Keeping them separate is what makes the registry free.

**Mechanism:** a static `LLM_PRESETS` registry (mirrors `models.rs::REGISTRY` in spirit) plus one new
optional field, `presetId: string`, used **for display only** — the badge label and which preset the
editor's select shows. It never changes request behavior; two profiles with `presetId: "groq"` and
`presetId: "custom"` at the same URL behave identically. Locality (`local`/`cloud`) stays _derived
from the base-URL host_ (00 §10; REFINE.md), not from `presetId` — a preset can be hand-edited, the
URL cannot lie.

### 1.2 The preset registry

Each preset carries `id`, `displayName`, `baseUrl`, the `kind` it maps to (`ollama` for Ollama,
`openaiCompatible` for everything else), auth style, a default model _suggestion_ (prefill only,
never locked), local/cloud, and a one-line caveat. Model suggestions drift — mark **(verify at
implementation)**.

| id           | Display name               | Base URL                                                  | kind               | Auth         | Default model (suggest)              | Loc   | Caveat                                                                                                                                                                                                                                                    |
| ------------ | -------------------------- | --------------------------------------------------------- | ------------------ | ------------ | ------------------------------------ | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ollama`     | Ollama                     | `http://localhost:11434`                                  | `ollama`           | none         | `qwen2.5:3b`                         | local | Native `/api/tags` model listing; the only special-cased provider.                                                                                                                                                                                        |
| `lmstudio`   | LM Studio                  | `http://localhost:1234/v1`                                | `openaiCompatible` | none         | (list from server)                   | local | Local server; start it and load a model first.                                                                                                                                                                                                            |
| `llamacpp`   | llama.cpp server           | `http://localhost:8080/v1`                                | `openaiCompatible` | none         | (whatever is loaded)                 | local | `llama-server` serves one model; `model` field is ignored by some builds.                                                                                                                                                                                 |
| `openai`     | OpenAI                     | `https://api.openai.com/v1`                               | `openaiCompatible` | Bearer key   | `gpt-4o-mini` _(verify)_             | cloud | Text-only leaves the Mac. Standard OpenAI keys.                                                                                                                                                                                                           |
| `groq`       | Groq                       | `https://api.groq.com/openai/v1`                          | `openaiCompatible` | Bearer key   | `llama-3.3-70b-versatile` _(verify)_ | cloud | OpenAI-compatible; very fast.                                                                                                                                                                                                                             |
| `openrouter` | OpenRouter                 | `https://openrouter.ai/api/v1`                            | `openaiCompatible` | Bearer key   | `openai/gpt-4o-mini` _(verify)_      | cloud | Router across many models; model ids are namespaced `vendor/model`.                                                                                                                                                                                       |
| `mistral`    | Mistral                    | `https://api.mistral.ai/v1`                               | `openaiCompatible` | Bearer key   | `mistral-small-latest` _(verify)_    | cloud | OpenAI-compatible chat endpoint.                                                                                                                                                                                                                          |
| `anthropic`  | Anthropic (Claude)         | `https://api.anthropic.com/v1`                            | `openaiCompatible` | Bearer key   | `claude-sonnet-4-x` _(verify)_       | cloud | OpenAI-compat layer is **eval-grade, not production** per Anthropic: no prompt caching / structured outputs / extended thinking; `strict` ignored; **audio stripped** (irrelevant — we send text). Refinement still works. **(verify at implementation)** |
| `gemini`     | Google Gemini              | `https://generativelanguage.googleapis.com/v1beta/openai` | `openaiCompatible` | Bearer key   | `gemini-2.x-flash` _(verify)_        | cloud | OpenAI-compat path supports chat + embeddings only; some clients wrongly hit `/responses` — we use `/chat/completions`, which is correct. **(verify at implementation)**                                                                                  |
| `custom`     | Custom (OpenAI-compatible) | _(empty — user fills)_                                    | `openaiCompatible` | optional key | _(empty)_                            | —     | Raw fields, exactly today's behavior. The escape hatch for anything unlisted (z.ai `/v4`, Together, Fireworks…).                                                                                                                                          |

The URL builder already handles every path shape these need: `chat_completions_url` trusts an
explicit path and only inserts `/v1` for a bare origin, with tests covering Groq's `/openai/v1`,
z.ai's `/v4`, and **Gemini's `/v1beta/openai`** (`llm.rs` tests, lines 289–309). No client change is
required to add presets — that is the whole point. Anthropic and Gemini facts confirmed against
vendor docs 2026-06-11 (sources in §9); model ids drift, hence the **(verify)** marks.

### 1.3 Profile editor UX

The REFINE.md editor (kept verbatim, re-homed to **Models → AI profiles** per 03 §2) gains one
behavior: the **Provider** select becomes a **preset** select.

- A preset **prefills** Base URL, `kind`-derived auth visibility, and the model _suggestion
  placeholder_ — and **never locks** any field (a user behind a proxy edits the URL freely).
- **Custom** shows raw fields, no prefill — identical to today's `openaiCompatible` form.
- **Changing the preset after editing** asks first, so a curated URL is never clobbered: _"Replace
  the fields with OpenAI's defaults? Your edits to Base URL and Model will be overwritten."_ If
  nothing was edited, re-prefill is silent.
- The model field offers suggestions but accepts anything. Ollama keeps its **"List installed
  models"** chips (the one native call); cloud presets have no portable list endpoint we trust, so
  free text with the suggestion as placeholder.

No new command needed — preset selection is front-end state until save, then the existing
`save_llm_profile` with `presetId` along for the ride.

---

## 2. STT side — a real engine abstraction

This is the new architecture. Today there is exactly one engine: in-process whisper.cpp behind a
`Mutex` in `spawn_blocking` (`stt.rs`), fed 16 kHz mono `f32` samples, biased by the dictionary as
`initial_prompt`, against a static ggml registry (`models.rs`). The seam ROADMAP.md already names —
"engine choice behind the existing `stt.rs` interface" — becomes a trait.

### 2.1 The `SttEngine` trait

Design goals: keep the local path exactly as fast as today (no boxing on the hot allocation, no
async on whisper), let cloud engines be plain async HTTP, and make capabilities _honest_ — whisper
biases vocabulary with `initial_prompt`, Deepgram with `keyterm`, AssemblyAI with `word_boost`;
the trait must not pretend these are the same call.

```rust
/// What an engine can do, queried by the pipeline before it builds options.
/// Honest surface: each field maps to a real provider capability, not a wish.
#[derive(Debug, Clone, Copy)]
pub struct SttCapabilities {
    pub language_hint: bool,        // accepts an ISO-639-1 hint ("en") or auto-detect
    pub word_timestamps: bool,      // can return per-word timing (unused today; reserved)
    pub vocabulary: VocabularyHint, // how, if at all, it biases toward known terms
    pub streaming: bool,            // supports incremental results (none shipped initially)
}

/// The vocabulary-biasing mechanism differs per provider; the pipeline picks
/// the right mapping from the dictionary instead of assuming `initial_prompt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyHint {
    None,
    InitialPrompt, // whisper.cpp, OpenAI/Groq audio `prompt` (≤ ~224 tokens)
    Keyterms,      // Deepgram Nova-3 `keyterm` (≤ 100 terms)
    WordBoost,     // AssemblyAI `word_boost` (≤ 1000 phrases, boost_param)
}

/// Where the transcript bytes physically go. The pipeline shows this to the
/// user; `Cloud` is the privacy gate's trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locality {
    Local,
    Cloud,
}

/// One transcription result. Mirrors today's `String` return plus room to grow
/// without another signature change.
pub struct Transcript {
    pub text: String,
    pub detected_language: Option<String>,
}

/// Options resolved per job (mode/profile resolution; see §4 and 07 §3).
pub struct TranscribeOptions<'a> {
    pub language: &'a str,            // "auto" or ISO-639-1
    pub vocabulary: &'a [DictionaryEntry], // engine maps these via its VocabularyHint
}

/// The engine seam. Local engines run the body inside `spawn_blocking`
/// (CPU/GPU heavy, `!Send` state stays put); cloud engines issue async HTTP.
/// Either way the result is an `AppResult` so failures become HUD notices.
#[async_trait::async_trait]
pub trait SttEngine: Send + Sync {
    fn id(&self) -> &str;                  // engine prefix, e.g. "whispercpp", "groq"
    fn display_name(&self) -> &str;
    fn locality(&self) -> Locality;
    fn capabilities(&self) -> SttCapabilities;

    /// Transcribe 16 kHz mono samples. Local: heavy, sync-in-spawn_blocking.
    /// Cloud: encodes a WAV and POSTs it. Must observe cancellation (§2.4).
    async fn transcribe(
        &self,
        samples_16k_mono: &[f32],
        opts: &TranscribeOptions<'_>,
    ) -> AppResult<Transcript>;
}
```

Notes on shape:

- **The local engine keeps its internals.** `WhisperCppEngine` wraps today's `SttEngine` struct
  (renamed, e.g. `WhisperContextCache`, to free the trait name); its `transcribe` body still runs in
  `spawn_blocking` behind the existing `Mutex`. The `#[async_trait]` wrapper just `spawn_blocking`s
  and awaits the join — an await point, not a thread move. Whisper stays off the executor (00 §8.4).
- **`&[f32]` stays the input** for both kinds. The pipeline already holds 16 kHz mono samples after
  `audio.stop()`; cloud engines encode that to a 16-bit PCM WAV in-process (payload math, §8) before
  upload. No format negotiation on the hot path.
- **`VocabularyHint` is the honest part.** The pipeline asks how the engine biases, then builds the
  matching payload: `initial_prompt_from_dictionary` for `InitialPrompt`, a `keyterm` list for
  Deepgram, a `word_boost` list for AssemblyAI. `None` engines drop the STT hint; the dictionary's
  `from → to` _replacement_ still runs in `text.rs` afterward (engine-independent post-processing).

### 2.2 Engine implementations

The critical column is **what audio leaves the machine** — everything else flows from it.

| Engine                              | Protocol                                                                           | Audio leaves the Mac?                                                                                                              | Auth                         | Latency (rough)                       | Vocabulary mapping                             | Failure modes                                                                     |
| ----------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------- |
| **whisper.cpp** (today)             | in-process whisper-rs, Metal                                                       | **No.** Nothing leaves.                                                                                                            | none                         | ~real-time on Apple Silicon           | `initial_prompt`                               | model file missing; OOM on huge model                                             |
| **whisper-server / Faster-Whisper** | local HTTP, OpenAI-audio-compatible `/v1/audio/transcriptions`                     | **No** (localhost) — but _is_ an HTTP hop, so a misconfigured non-local URL would leak; locality is derived from host, not assumed | usually none                 | ~real-time, depends on host hardware  | `prompt` (OpenAI-audio field)                  | server down; wrong port; model not loaded                                         |
| **OpenAI Whisper API**              | cloud HTTP `/v1/audio/transcriptions`                                              | **Yes — full audio.**                                                                                                              | Bearer key                   | network + ~1–3 s                      | `prompt` (≤ ~224 tokens)                       | 401; 429 rate limit; **25 MB cap**; timeout                                       |
| **Groq** (hosted whisper-large-v3)  | cloud HTTP, OpenAI-audio-compatible `/openai/v1/audio/transcriptions`              | **Yes — full audio.**                                                                                                              | Bearer key                   | network + sub-second (very fast)      | `prompt` (≤ ~224 tokens)                       | 401; 429; **25 MB cap** (use `url` param above that — we never will, §8); timeout |
| **Deepgram**                        | cloud HTTP, **bespoke** `/v1/listen` prerecorded                                   | **Yes — full audio.**                                                                                                              | `Authorization: Token <key>` | network + sub-second                  | `keyterm` (Nova-3, ≤ 100) or legacy `keywords` | 401; 400 on bad params; model-name drift                                          |
| **AssemblyAI**                      | cloud HTTP, **bespoke** two-step: `POST /v2/upload` → `POST /v2/transcript` → poll | **Yes — full audio.**                                                                                                              | `authorization: <key>`       | network + several seconds (poll loop) | `word_boost` (≤ 1000 phrases, `boost_param`)   | 401; poll latency; partial-result handling                                        |

**The collapse.** whisper-server, Faster-Whisper, OpenAI, and Groq all speak the **same**
OpenAI-audio multipart shape (`file`, `model`, `prompt`, `language`, `response_format`) — **one
generic client**, the exact mirror of the LLM-side "one client for all providers" decision. Call it
`OpenAiAudioEngine`, parameterized by base URL + key + model + locality, reusing the path-trusting URL
builder (`/openai/v1`, `/v1` both work). Deepgram and AssemblyAI need **bespoke** clients: different
endpoints, auth header names, bodies, and AssemblyAI's upload-then-poll dance. So the generic client
unlocks four engines in one implementation; the two bespoke ones are separate, later builds (§7) — do
the generic one first.

### 2.3 Engine "connections": `SttProfile`, a sibling of LLM profiles — not a unified `kind`

Cloud STT engines need URL + key config — the same shape `LlmProfile` already stores. The decision:

> **Reuse the EXACT file-backed pattern from `profiles.rs`, in a sibling directory
> `<app-data>/stt-profiles/`, with its own `SttProfile` struct.** Do **not** add a `kind` field to a
> single unified profile type.

Why a sibling directory and a separate struct, not one unified file with a discriminant:

- **File-format clarity beats code reuse here.** A unified `{kind: "llm" | "stt", …}` file commingles
  two domains' fields (an LLM profile has no `engine`; an STT profile has no chat-only fields) and
  makes every reader branch on `kind`. The scan/atomic-write/0600/skip-corrupt core is ~60 lines
  (`profiles.rs`); copying that tiny, well-tested core is cheaper and clearer than the branching. Per
  repo rules, we do not add a discriminant abstraction for two call sites when copying the loader is
  clearer.
- **Symmetry the user can see.** Two directories, two lists, same idiom — `ProfileManager` is a small
  generic or just instantiated twice over `LlmProfile` and `SttProfile`. Show in Finder, hand-dropped
  files on next scan, skip-corrupt-never-delete — all inherited verbatim.
- **The default needs no profile.** Local whisper.cpp is the zero-config default — an _engine_ picked
  by the speech-model radio, **not** a "profile." Profiles exist only for engines needing URL + key
  (the cloud four, plus a whisper-server on a non-default port). Mirrors REFINE.md's "No AI" = absence
  of a profile.

```jsonc
// <app-data>/stt-profiles/9a1c….json   (0600, atomic, schema-versioned)
{
  "version": 1,
  "id": "9a1c…", // filename stem is identity (same rule as LlmProfile)
  "name": "Groq Whisper",
  "engine": "openaiAudio", // which client: "openaiAudio" | "deepgram" | "assemblyai"
  "presetId": "groqStt", // display/prefill only, like the LLM side
  "baseUrl": "https://api.groq.com/openai/v1",
  "apiKey": "gsk_…",
  "model": "whisper-large-v3",
  "timeoutSecs": 30,
}
```

`engine` here is the bespoke-vs-generic selector (the STT analogue of `LlmProviderKind`): three
values, because there are genuinely three clients. It is small and behavioral, unlike `presetId`.

### 2.4 Threading and cancellation

- **Local engines** run inside `spawn_blocking` exactly as today; the `Mutex` over the whisper
  context is untouched. No new thread, no executor move (00 §8.4).
- **Cloud engines** are plain `async` HTTP via the existing `reqwest::Client` (already used by
  `llm.rs` and `models.rs`). They run on the Tokio runtime like the LLM call already does in
  `finish_dictation`. No audio-thread or output-thread involvement.
- **Cancellation must abort or orphan cloud uploads.** The generation counter (`pipeline.rs`) already
  works for the LLM call: `cancel()` bumps the generation; every async stage re-checks before
  publishing (`process` → `if generation != … return`). Cloud transcription is the same shape. Two
  refinements: (1) **orphaning is the baseline** — a bumped generation discards the result the instant
  it returns, with no UI effect; (2) **best-effort abort** by dropping the future / holding it in a
  `tokio::select!` against a per-generation cancel signal, so a cancelled 9 MB upload stops sending.
  Either way **a cancelled job never inserts**, and (see §3) abandoning a cloud upload is the _good_
  direction — less audio leaves, not more.

### 2.5 Model registry impact

`models.rs` is whisper-ggml-specific (HF URLs, `.bin` files, sizes). The abstraction makes the Models
page list **per-engine models**:

- **Local engines** keep the download story: `REGISTRY` entries are _whisper.cpp_ models —
  downloadable files with sizes, install/delete, multilingual badge. A future local engine (Parakeet,
  ROADMAP.md) brings its own static sub-registry.
- **Cloud engines** have no files. Their "models" are **names** — a short static list per engine
  (`whisper-large-v3`/`-turbo` for Groq; `nova-3` for Deepgram) or free text. No size, no install
  state; the row shows the locality badge and a model-name field, not a download button.

So Speech recognition becomes **engine-grouped**: whisper.cpp on top (default, zero-config), then any
configured cloud/remote engines with their model field. Wireframe in §5.

---

## 3. The privacy gate — cloud STT sends audio

This is the load-bearing section. Cloud STT crosses Velata's core promise. The README/PRD say
"audio never leaves your Mac"; an OpenAI/Groq/Deepgram/AssemblyAI engine **uploads the full
recording**. This stays _within_ 00 §8.1 (cloud = opt-in, BYO-key) — but only if the consent is
loud, the locality is always visible, and the docs stop over-claiming.

### 3.1 A stronger badge than the LLM side

The STT engine row needs a **stronger** badge than the LLM side, because _audio_ is categorically
more sensitive than refined text:

- LLM cloud profile badge: **`cloud`** — "Refined text — never audio — is sent to this endpoint"
  (REFINE.md, kept).
- STT cloud engine badge: **`cloud — audio leaves this Mac`** — visually heavier (filled, warning
  color), never shortened to just `cloud`. Derived from `Locality::Cloud`, which the host check
  downgrades for a localhost whisper-server (→ `Local`).

### 3.2 First-activation consent dialog (exact copy)

Selecting a `Cloud` STT engine's radio for the **first time** (per engine) opens a blocking
confirmation. It must name _what_ is sent, _to whom_, and that this _differs from the default_:

> **Title:** Send your audio to a cloud service?
>
> **Body:** Velata normally transcribes speech entirely on your Mac — your audio never leaves the
> device. **{EngineName}** is different: each time you dictate, the full audio recording is uploaded
> to **{provider host}** over the internet for transcription, using your own API key.
>
> Your audio is sent to a third party you've chosen. Velata does not store it, but what that
> service does with it is governed by _their_ policy, not Velata's. The on-device whisper engine
> remains available and stays the default.
>
> **Buttons:** `[ Keep on-device ]` `[ Use {EngineName} (uploads audio) ]`

The confirmed-engines set is persisted (a small `confirmedCloudSttEngines: string[]` in settings, or
a flag per `SttProfile`) so the dialog appears once per engine, not every switch. Switching _away_
to a local engine never prompts. Adding a _new_ cloud profile of an already-confirmed engine kind
still re-confirms, because it is a new endpoint/key (a new place audio goes).

### 3.3 Always-visible active locality

Consent at switch time is not enough; the user must see it _while it is happening_:

- **Tray.** Under the mode-list radios, a non-interactive status label reflecting the active engine:
  **"Speech: on-device"** (default) or **"Speech: cloud — {EngineName}"** with the warning glyph. A
  disabled `MenuItem`, not a control — engines are switched on the Models page (config, not a
  quick-toggle; 03 §4).
- **HUD.** With a cloud engine active, the `recording`/`transcribing` label carries a glyph — e.g.
  **"Transcribing… ⛅ cloud"** — so the privacy state shows where the user is already looking (09 owns
  the visual; this doc owns the requirement). The HUD stays click-through, never shown/hidden (00
  §8.2) — only content changes.

### 3.4 Required docs change (call it out)

A **required** change shipping with any cloud STT engine. The absolute claim is reworded; the
_invariant_ is intact (nothing leaves by default, cloud opt-in BYO-key, no servers, nothing stored) —
only the _prose_ changes, which currently over-promises for a build that can do cloud STT.

- **README / PRD / Models privacy line.** Replace "audio never leaves your Mac" with: _"By default,
  Velata transcribes entirely on your Mac and your audio never leaves the device. Cloud speech
  engines are off unless you add one yourself (bring your own key); turning one on uploads your audio
  to that provider, and Velata tells you clearly before it does."_
- **PRIVACY.md** gains a "Cloud speech engines" subsection: opt-in only, BYO-key, audio uploaded per
  dictation to the chosen provider, Velata stores nothing, the provider's policy governs the audio,
  on-device stays the default.
- **Models Speech card summary line** flips with the active engine: default → "Speech is processed on
  your Mac."; cloud → "Speech is sent to {EngineName} (cloud)."

---

## 4. Pipeline integration

### 4.1 Where engine resolution happens

Engine resolution happens **at job start**, alongside mode/profile resolution (07 §3) — where
`process()` already reads `settings.stt_model_id`, the active mode, and the active LLM profile. In
`process()` (today `pipeline.rs:329`), the bare `stt.transcribe(model_id, …)` becomes: (1) resolve
the effective speech model id (per-mode `sttModelId` override or global — 07 §3); (2) split into
`(enginePrefix, modelName)` (§4.2); (3) look up the engine (whisper.cpp always present; cloud engines
from the matching `SttProfile`); (4) build `TranscribeOptions` from `language` + dictionary via the
engine's `VocabularyHint`; (5) `engine.transcribe(&samples, &opts).await`.

Local engines `spawn_blocking` internally (§2.4); cloud engines await HTTP. The post-await generation
re-check is unchanged. Engine choice is _per job_, so a mode overriding into a cloud engine (07)
transcribes through it for that dictation only.

### 4.2 Engine-qualified model ids and migration

Once engines exist, a bare `base.en` is ambiguous. **Model ids become engine-qualified**,
`engine:model`: `whispercpp:base.en`, `openaiAudio:whisper-large-v3` (the _profile_ pins endpoint/key;
a per-provider alias like `groq:whisper-large-v3` is a UI nicety the registry can resolve),
`deepgram:nova-3`, `assemblyai:best`.

**Migration of bare ids** (additive, automatic, mirrors REFINE.md's self-erasing migration): in
`normalize()`/`reconcile`, a `sttModelId` with **no `:`** is treated as whisper.cpp and rewritten to
`whispercpp:<id>` on first load, then persisted (dropping the bare form). An unknown prefix or a
vanished model clears to the default `whispercpp:base.en` with a log warning — same shape as the
dangling-active-profile rule in `profiles.rs::reconcile`. Per-mode `sttModelId` overrides (07) get the
same rewrite.

### 4.3 Failure policy — never silent fallback toward more data leaving

Cloud engine unreachable, 401, 429, or timeout → **error notice naming the fix**, never a silent
swap. The rule, stated precisely:

> **Automatic fallback may only move in the direction of _less_ data leaving the machine
> (cloud → local-rules), never _toward_ it (local → cloud).**

- **Dictation, cloud STT fails:** the recording is already on-device in memory. We _may_ fall back
  to **local whisper.cpp** automatically _only if_ the user has a whisper model installed, because
  that direction sends _less_ data, not more — then a notice: "Cloud transcription failed; used the
  on-device model instead." If no local model is installed, error notice: "Cloud transcription failed
  ({reason}). Switch to the on-device model in Settings." Dictation output is never silently dropped
  (00 §8.6) — worst case the user re-dictates; we never invent text.
- **Never the reverse.** A local whisper failure (e.g. model deleted mid-session) must **never**
  auto-escalate to cloud — that pushes audio off the machine without consent. It errors with the
  existing "model not downloaded" guidance.
- **Selection jobs (Rewrite/Polish)** keep their deliberate no-fallback (REFINE.md): selection
  untouched, error shown. Unchanged.

This mirrors dictation's existing "LLM error → rules cleanup + notice" policy, plus the directional
constraint unique to audio.

---

## 5. Provider-management UX — Models page wireframe

The Models page (03 §2) holds two cards. Speech recognition is now engine-grouped; AI profiles is the
in-flight design plus the preset select.

```
┌─ Models ─────────────────────────────────────────────────────────────────────┐
│                                                                               │
│  Speech recognition                                          [ Add engine… ]  │
│  Speech is processed on your Mac.                                             │  ← summary line, flips per active engine
│                                                                               │
│   On-device (whisper.cpp)                                  local              │
│     (•) Base (English)            148 MB   Fast, decent accuracy   [Delete]   │  ← radio = active model; downloadable files
│     ( ) Small (English)           488 MB   ⤓ Download                          │
│     ( ) Large v3 Turbo (Q5)       574 MB   ⤓ Download                          │
│                                                                               │
│   Groq Whisper              cloud — audio leaves this Mac   [Edit] [Delete]   │  ← a configured cloud engine (SttProfile)
│     ( ) whisper-large-v3                       (model name field)              │  ← no size, no download; radio selects it
│                                                                               │
│   ── empty state (no cloud engines) ──                                        │
│   Add a cloud speech engine (OpenAI, Groq, Deepgram, AssemblyAI) for faster   │
│   or higher-accuracy transcription. Cloud engines upload your audio — you'll  │
│   confirm before any audio is sent.                            [ Add engine… ] │
│                                                                               │
│ ┌─ Add / Edit speech engine (sheet) ─────────────────────────────────────┐   │
│ │  Name        Groq Whisper                                              │   │
│ │  Engine      OpenAI-compatible audio ▾   (OpenAI-compat | Deepgram |   │   │
│ │                                           AssemblyAI)                  │   │
│ │  Preset      Groq ▾  (prefills URL; never locks)                       │   │
│ │  Base URL    https://api.groq.com/openai/v1                            │   │
│ │  API key     ••••••••••••                                              │   │
│ │  Model       whisper-large-v3                                          │   │
│ │  Timeout     30 s                                                      │   │
│ │                                  [ Test ]   [ Cancel ]   [ Save ]      │   │
│ └────────────────────────────────────────────────────────────────────────┘   │
│                                                                               │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                               │
│  AI profiles                              [ Show in Finder ]  [ New profile ] │  ← REFINE.md, verbatim + preset select
│   ( ) No AI — rules-based cleanup only                                        │
│   (•) Ollama qwen2.5        local — qwen2.5:3b                                 │
│   ( ) Groq Llama            cloud — llama-3.3-70b              [Edit] [Delete] │
│                                                                               │
│   ── empty state ──                                                           │
│   Dictation uses fast rules-based cleanup. Add a profile to enable AI polish  │
│   and the selection shortcuts.                              [ New profile ]   │
│                                                                               │
│ ┌─ Edit profile (sheet) ──────────────────────────────────────────────────┐  │
│ │  Name        Groq Llama                                                 │  │
│ │  Provider    Groq ▾        (preset: prefills URL/auth; never locks)     │  │
│ │  Base URL    https://api.groq.com/openai/v1                             │  │
│ │  API key     ••••••••••••                                               │  │
│ │  Model       llama-3.3-70b-versatile                                    │  │
│ │  Timeout     30 s                                                       │  │
│ │  ⓘ Refined text — never audio — is sent to this endpoint.  (cloud only) │  │
│ │                                       [ Test connection ]   [ Delete ]  │  │
│ └─────────────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────────────┘
```

### 5.1 Exact copy

- **STT cloud badge:** `cloud — audio leaves this Mac`. **LLM cloud badge:** `cloud`. **Local badge
  (both):** `local`.
- **STT empty state:** "Add a cloud speech engine (OpenAI, Groq, Deepgram, AssemblyAI) for faster or
  higher-accuracy transcription. Cloud engines upload your audio — you'll confirm before any audio is
  sent."
- **AI profiles empty state (REFINE.md, kept):** "Dictation uses fast rules-based cleanup. Add a
  profile to enable AI polish and the selection shortcuts."
- **Consent dialog:** §3.2 verbatim.
- **Speech summary line:** default "Speech is processed on your Mac." / cloud active "Speech is sent
  to {EngineName} (cloud)."

### 5.2 Test buttons — both sides, no mic permission needed

- **LLM Test** (`test_llm`, exists): one `chat` round-trip ("reply OK"), inline result. Unchanged.
- **STT Test** (`test_stt`, new): **transcribe a ~1-second bundled PCM sample**, not the live mic.
  _Justification:_ the settings window must not require Microphone permission to test a connection,
  and under `pnpm dev` mic grants attach to the terminal not the app (00 macOS notes) — an embedded
  sample sidesteps TCC entirely. Ship a tiny mono 16 kHz WAV of a known phrase ("testing one two
  three", a few KB); `test_stt` feeds its samples to the engine and reports success + returned text,
  or the `AppError`. It doubles as a real end-to-end check — auth, endpoint, model name, vocabulary
  mapping all on the dictation path. Local whisper.cpp's test still needs the model file installed
  (nothing to upload).

---

## 6. IPC + schema impact

Both sides in the same PR (00 §8.3). All additive where possible; existing fields keep working
through migration.

### 6.1 New / changed TypeScript mirrors (sketch)

```ts
// LLM side — presetId is additive; LlmProviderKind unchanged (no new variants).
export interface LlmProfile {
  version: number;
  id: string;
  name: string;
  provider: 'ollama' | 'openaiCompatible';
  presetId: string; // NEW: display/prefill hint only ("" = none/custom)
  baseUrl: string;
  apiKey: string;
  model: string;
  timeoutSecs: number;
}

// STT side — new types.
export type SttEngineKind = 'openaiAudio' | 'deepgram' | 'assemblyai';
export type Locality = 'local' | 'cloud';

export interface SttProfile {
  // sibling of LlmProfile, own directory
  version: number;
  id: string;
  name: string;
  engine: SttEngineKind;
  presetId: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  timeoutSecs: number;
}

// Engine descriptor the Models page renders (local + cloud, unified view).
export interface SttEngineInfo {
  id: string; // "whispercpp" | profile id for cloud engines
  displayName: string;
  locality: Locality;
  models: SttModelInfo[];
}

export interface SttModelInfo {
  id: string; // engine-qualified, e.g. "whispercpp:base.en"
  displayName: string;
  installed: boolean; // always true for cloud
  downloadable: boolean; // true only for local file models
  sizeBytes: number | null; // null for cloud
  multilingual: boolean;
}

export interface SttTestResult {
  ok: boolean;
  message: string;
  text: string | null;
}
```

### 6.2 Settings fields

| Field                                | Change                                                                    |
| ------------------------------------ | ------------------------------------------------------------------------- |
| `sttModelId`                         | now **engine-qualified** (`whispercpp:base.en`); bare ids migrated (§4.2) |
| `confirmedCloudSttEngines: string[]` | NEW, default `[]` — which cloud engines passed the consent dialog (§3.2)  |
| `version`                            | bump (REFINE.md set v2; this is **v3**) with the §4.2 + consent migration |

Per-mode `sttModelId` override (07) becomes engine-qualified by the same migration rule. No other
mode-shape change.

### 6.3 Commands

| Command                                         | Change                                                                                                               |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `list_stt_engines` → `SttEngineInfo[]`          | NEW; whisper.cpp + configured cloud engines, with their models                                                       |
| `list_stt_profiles` → `SttProfile[]`            | NEW; rescans `<app-data>/stt-profiles/` (mirror of `list_llm_profiles`)                                              |
| `save_stt_profile(profile)` → `SttProfile[]`    | NEW; upsert, returns fresh list                                                                                      |
| `delete_stt_profile(id)` → `SttProfile[]`       | NEW                                                                                                                  |
| `reveal_stt_profiles`                           | NEW; opener `reveal_item_in_dir` (mirror of `reveal_llm_profiles`)                                                   |
| `test_stt(profileOrEngineId)` → `SttTestResult` | NEW; transcribes the bundled sample (§5.2)                                                                           |
| `save_llm_profile` / `LlmProfile`               | additive `presetId` field; no signature change                                                                       |
| `save_settings`                                 | now also validates `sttModelId` engine-qualification; clears the consent set entry if an engine's profile is deleted |

Active speech-engine/model selection stays `save_settings` writing `sttModelId` (no dedicated
command), mirroring how active LLM profile is just `activeLlmProfileId` (REFINE.md). The
`settings-changed` event already broadcasts it; profile mutations return the list, no new event.

---

## 7. Shipping order

Phased so each phase ships value and the trait lands exactly when the first engine needs it.

| Phase                                     | Scope                                                                                                                                                                                                                                                                                                                                          | Effort           | Unlocks                                                                                                                         |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **P1 — LLM presets**                      | `LLM_PRESETS` registry, `presetId` field, preset select + confirm-on-change in the editor. **Pure UI + one additive field.** No client change (URL builder already covers every path).                                                                                                                                                         | days             | Correct config for OpenAI, Anthropic, Gemini, Groq, OpenRouter, Mistral, LM Studio, llama.cpp out of the box; Custom unchanged. |
| **P2 — STT trait + generic audio client** | Land the `SttEngine` trait; wrap whisper.cpp as `WhisperCppEngine` (no internal change); build **one** `OpenAiAudioEngine`; `SttProfile` + `stt-profiles/` dir (copy the `profiles.rs` core); engine-qualified ids + migration; the **privacy gate** (badge, consent dialog, tray/HUD locality, docs rewrite); `test_stt` with bundled sample. | the bulk         | whisper-server, Faster-Whisper, OpenAI Whisper API, **and Groq** — all four from the single generic client.                     |
| **P3 — bespoke engines**                  | `DeepgramEngine` (`/v1/listen`, `keyterm`, Token auth) and `AssemblyAiEngine` (upload→transcript→poll, `word_boost`). Each is one engine impl + one preset; the trait, profiles, UX, and gate already exist.                                                                                                                                   | each independent | Deepgram, AssemblyAI.                                                                                                           |

The trait **lands in P2**, not P1 — P1 needs no trait (it is the existing single client with better
prefill). Putting the trait in P2 means it is born with two real implementors (local whisper + generic
cloud), which is the right number to design a trait against.

### What NOT to build until asked (00 §8.8, ROADMAP non-goals)

- **No plugin system, dynamic loading, `dlopen`/WASM modules, marketplace, or remote catalog.**
  Engines are compiled-in Rust behind a static registry, exactly like `models.rs`. The trait is
  _internal_ extensibility; third-party engines are out of scope.
- **No streaming STT yet** — `SttCapabilities.streaming` is reserved, nothing ships it.
- **No accounts, sync, or telemetry** to "manage" providers (00 §8.1, §8.8). Config is local files.

---

## 8. Edge cases

- **Engine removed while a mode overrides into it (dangling ref).** Same rule as the LLM dangling
  pointer (`profiles.rs::reconcile`) and 07 §3: the mode's `sttModelId` resolves **inherit → global
  default** (whisper.cpp) with a one-time log warning; the mode is not mutated on disk unless edited.
  Never errors the dictation — falls back to on-device (the _less data leaves_ direction, §4.3).
- **Key revoked mid-session.** First dictation after revocation gets `401` → §4.3: fall back to
  on-device whisper if installed (notice), else error naming the fix. That attempt's audio was
  uploaded and rejected; nothing is stored. One attempt, one fallback — no retry storm.
- **Model file deleted on disk (local).** `start()` checks `is_installed`; a mid-session delete
  surfaces as the whisper load error. The engine **never** auto-escalates to cloud (§4.3); the notice
  points to Settings to re-download.
- **Two engines claim the same model id.** Impossible once ids are engine-qualified (§4.2): the prefix
  disambiguates, and bare legacy ids always migrate to `whispercpp:`. No ambiguous state is reachable.
- **Cancellation racing a cloud upload.** Generation counter + best-effort abort (§2.4): the bump
  discards the result; the in-flight POST is dropped/`select!`-cancelled so the upload stops. A
  cancelled job never inserts, and abandoning the upload sends _less_ audio — the safe direction.
- **Giant audio vs cloud payload caps.** Hard cap is **300 s** (`MAX_RECORDING_SECS`) at 16 kHz mono.
  As 16-bit PCM WAV that is `300 × 16000 × 2 ≈ 9.6 MB` (+44 B header); raw `f32` is ~19.2 MB, but we
  encode 16-bit for upload. **9.6 MB is well under every cap:** OpenAI **25 MB**, Groq **25 MB** (and
  its 1500 s duration limit — we're at 300 s), Deepgram/AssemblyAI GB-scale. So the 5-minute cap
  _already_ guarantees we never hit a size limit on any listed provider — no chunking, ever, on the
  dictation path. (A future longer-recording mode would make OpenAI/Groq's 25 MB binding at ~13 min of
  16-bit mono; revisit then.)
- **Local whisper-server on a non-local host.** Locality is derived from the URL host, not the engine
  kind — a "local" whisper-server profile with a non-loopback `https://` URL is treated as **cloud**
  (badge + consent fire). The host check is the single source of truth, as on the LLM side.

---

## 9. Cross-references

- **00 §8** — privacy invariants (§3), threading §8.4 (§2.4), IPC mirror §8.3 (§6), failure policy
  §8.6 (§4.3), no-marketplace §8.8 (§7).
- **03 §2/§4** — Models page as home for both cards; tray = quick-switch, config in the window (§3.3, §5).
- **07 §3** — mode overrides, per-mode `sttModelId`, dangling-reference resolution (§4, §8).
- **09** — owns the HUD/badge visuals whose _requirements_ §3.3 sets.
- **REFINE.md / `profiles.rs`** — the `LlmProfile` pattern reused for `SttProfile`, extended with
  `presetId` (§1, §2.3). **ROADMAP.md** — "alternative STT engines behind the `stt.rs` interface" is
  this seam; whisper-server sidecar is the generic client's local case.

### Source facts (verified 2026-06-11; model ids drift — verify at implementation)

- OpenAI `/v1/audio/transcriptions`, 25 MB file cap, `prompt`/`language` params:
  https://developers.openai.com/api/docs/guides/speech-to-text ·
  https://help.openai.com/en/articles/7031512-audio-api-faq
- Groq OpenAI-compatible audio `https://api.groq.com/openai/v1/audio/transcriptions`, 25 MB cap,
  `prompt` ≤ 224 tokens, whisper-large-v3 / -turbo: https://console.groq.com/docs/speech-to-text
- Deepgram prerecorded `/v1/listen`, `Authorization: Token`, Nova-3 `keyterm` (≤100):
  https://developers.deepgram.com/docs/keyterm · https://developers.deepgram.com/docs/pre-recorded-audio
- AssemblyAI two-step `/v2/upload` → `/v2/transcript`, `authorization` header, `word_boost` (≤1000,
  `boost_param`): https://docs.assemblyai.com/core-transcription ·
  https://www.assemblyai.com/docs/faq/are-there-any-limits-on-file-size-or-file-duration-for-files-submitted-to-the-api
- Anthropic OpenAI-compat `https://api.anthropic.com/v1/` (eval-grade; audio stripped):
  https://docs.anthropic.com/en/api/openai-sdk
- Gemini OpenAI-compat `https://generativelanguage.googleapis.com/v1beta/openai/` (chat + embeddings,
  Bearer): https://ai.google.dev/gemini-api/docs/openai
