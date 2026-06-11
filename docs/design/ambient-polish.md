# The small stuff — sound, focus, presence, language

Status: **partially shipped**. "Show in Dock" is built (a `showInDock` setting that switches the
macOS activation policy between Regular and Accessory, applied at startup, on save, and on window
close). The rest — start/stop sounds, focus ducking, completion notifications, and UI
localization — remain deliberate follow-ups: each needs new infrastructure (audio playback, the
notification plugin + capability, an audio-session API, or an i18n string-table track) that
warrants its own focused change rather than riding this one.

## Why

A menu-bar tool lives in peripheral vision, and a handful of small, expected affordances are what
make it feel finished rather than minimal. None of these is a headline feature; together they
close the gap between "works" and "polished." Each is small enough to land independently.

## The grab-bag

### 1. Start / stop sounds

You are not always looking at the HUD when you start talking — especially mid-task, eyes on the
document. A short, optional tone on record-start and on insert confirms the app heard you. **Off
by default** (the HUD is the primary signal); a single enable toggle, maybe a volume. Already
noted as a want; cheap with a bundled tone or a system sound. Touches `audio.rs` (start) and the
pipeline's insert-success point (stop).

### 2. Focus ducking

When you start recording, briefly lower _other_ apps' audio — a podcast, music — so the mic is
not fighting a soundtrack, then restore it on stop. Improves transcription quality in the exact
moment it matters and feels considerate. Opt-in; touches the macOS audio session and needs care
to always restore (even if the job errors or is cancelled). The restore-on-all-exits requirement
makes this the fiddliest item here despite sounding trivial.

### 3. Completion notifications (fallback-only)

The HUD is glanceable but transient. When something the user might _miss_ happens — insertion
fell back to the clipboard, or the frontmost app changed before paste — a single native
notification ("Your dictation is on the clipboard — press ⌘V") prevents lost output. Deliberately
**scoped to the cases worth interrupting for**, not every dictation, so it never becomes noise.
Complements, not replaces, the HUD and the tray's "Copy last dictation."

### 4. Menu-bar and Dock presence

Small expected toggles for people who like to tune their setup:

```
  Show app in Dock                         [ off ]   ← menu-bar-only by default
  Keep the recording indicator visible     [ off ]   ← always-on status vs. on-demand
```

`Show in Dock` flips the activation policy (accessory ↔ regular) in `main.rs`. The indicator
toggle ties into the menu-bar recording-icon idea (swap the template icon while recording).

### 5. UI language

Dictation languages exist, but the app _chrome_ — settings, HUD labels, onboarding — is
English-only. A non-native speaker can dictate Chinese yet must read an English settings window.
An `appLanguage` setting plus string tables would localise the interface itself. This is the
largest item in the bag (it is an i18n track, not a toggle) and should be scoped on its own; the
others are afternoons.

## Settings shape

A handful of additive fields, all camelCase-mirrored, most read by an existing module:

```jsonc
{
  "sounds": { "enabled": false, "volume": 0.5 }, // audio.rs + insert point
  "duckOtherAudio": false, // audio session
  "notifyOnFallback": true, // output.rs / pipeline
  "showInDock": false, // main.rs activation policy
  "appLanguage": "en", // string tables (own track)
}
```

No new IPC commands — these ride `save_settings`. Notifications use the OS notification plugin;
everything else is local OS configuration.

## Privacy fit

All local. Sounds and ducking touch only the audio session; notifications are local user-facing
messages; Dock/indicator are window configuration; UI language is string lookup. Nothing here
adds a network call or persists anything beyond a few settings flags the user set.

## Open questions

- Bundled tones vs. system sounds — bundled keeps the brand consistent and avoids a jarring
  default alert; system sounds are zero-asset.
- Ducking robustness: the restore path must survive cancel, error, and quit. If it cannot be made
  bulletproof, ship sounds without ducking rather than risk leaving a user's music muted.
- Notification scope: keep it to genuine "you might lose this" moments; a notification per
  successful dictation would be intolerable and is explicitly _not_ the proposal.
- Localization scope: which languages first, and whether to commit to the i18n maintenance
  burden at all before the rest of v2 settles. The dictation languages already shipped are a
  guide to demand.
