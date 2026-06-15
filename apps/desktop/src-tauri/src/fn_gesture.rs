//! Observing the `fn` (🌐) key as push-to-talk / hands-free triggers.
//!
//! The `fn` key is a MODIFIER, not an ordinary key: it never produces a key-down
//! event, only flips the `kCGEventFlagMaskSecondaryFn` bit in a `flagsChanged`
//! event. So we cannot register it through Carbon `RegisterEventHotKey` (the
//! global-shortcut path) — the only way to observe it is a `CGEventTap`, which
//! requires the Input Monitoring permission (the lowest-privilege of the event
//! taps, since we listen only and never alter the stream).
//!
//! Two layers live here:
//!   * [`GestureState`] — pure timing logic (no OS calls): it turns a stream of
//!     `(FnEdge, timestamp)` into [`Gesture`]s. Generic over the clock (the
//!     caller stamps each edge), so it is exhaustively unit-tested with integer
//!     milliseconds.
//!   * the `macos` module — a listen-only `CGEventTap` on a dedicated
//!     `velata-fn` thread with its own `CFRunLoop`, plus the Input Monitoring
//!     permission calls. Runtime behavior is manual-verify (a tap needs a TCC
//!     grant and real key presses); this layer is compiled and clippy-checked.

use std::time::Duration;

/// A raw transition of the `fn` modifier bit: pressed (`Down`) or released
/// (`Up`). Diffed from the `MaskSecondaryFn` bit on each `flagsChanged` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnEdge {
    Down,
    Up,
}

/// A recognized gesture emitted by [`GestureState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// The first press of a (possible) hold — fired immediately on `Down` so
    /// push-to-talk feels instant.
    Press,
    /// The release that ends a hold: the `Up` matching a press held at least
    /// [`HOLD_THRESHOLD`]. A sub-threshold tap emits no `Release` (it is a
    /// candidate for a double-tap instead).
    Release,
    /// A second press within [`DOUBLE_TAP_GAP`] of a sub-threshold tap.
    DoubleTap,
}

/// A press held at least this long is a hold (its release emits [`Gesture::Release`]);
/// shorter is a tap. Mirrors the pipeline's `TAP_THRESHOLD` so the fn path and the
/// accelerator fallback agree on what counts as a tap.
pub const HOLD_THRESHOLD: Duration = Duration::from_millis(350);

/// A second `Down` within this gap of a tap's `Up` is a double-tap.
pub const DOUBLE_TAP_GAP: Duration = Duration::from_millis(400);

/// What the previous, now-completed, press left behind — the only state a new
/// edge needs to classify itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prev {
    /// No press is open and none is pending as a double-tap candidate.
    Idle,
    /// A press is currently held (we saw `Down` at this time, no `Up` yet).
    Down { at: Duration },
    /// The last press was a sub-threshold tap that ended at this time — a `Down`
    /// within [`DOUBLE_TAP_GAP`] of it is a double-tap.
    PendingTap { up_at: Duration },
    /// A double-tap fired; its second press's `Up` must be swallowed so the
    /// double-tap is not also read as a (sub-threshold) hold.
    SwallowUp,
}

/// Pure `fn`-gesture recognizer. Feed it `(edge, timestamp)` in order; each call
/// returns at most one [`Gesture`]. The timestamps must be monotonic; the OS
/// layer stamps them from the event so double-tap-gap accuracy does not depend
/// on scheduling latency.
#[derive(Debug)]
pub struct GestureState {
    prev: Prev,
    hold: Duration,
    gap: Duration,
}

impl Default for GestureState {
    fn default() -> Self {
        Self::new(HOLD_THRESHOLD, DOUBLE_TAP_GAP)
    }
}

impl GestureState {
    pub fn new(hold: Duration, gap: Duration) -> Self {
        Self {
            prev: Prev::Idle,
            hold,
            gap,
        }
    }

    /// Classifies one `fn` edge. See the module decision table in tests.
    pub fn on_edge(&mut self, edge: FnEdge, now: Duration) -> Option<Gesture> {
        match edge {
            FnEdge::Down => self.on_down(now),
            FnEdge::Up => self.on_up(now),
        }
    }

    fn on_down(&mut self, now: Duration) -> Option<Gesture> {
        // A `Down` is a double-tap only when the immediately preceding press was
        // a sub-threshold tap whose `Up` is within the gap. A hold's `Up` and a
        // swallowed `Up` both reset to Idle, so a quick re-press after either is
        // a fresh Press — never a double-tap (the gap can exceed the hold
        // threshold, so a down-to-down comparison alone would false-positive).
        let double_tap =
            matches!(self.prev, Prev::PendingTap { up_at } if now.saturating_sub(up_at) < self.gap);
        if double_tap {
            // The second press's release must not also count as a hold release.
            self.prev = Prev::SwallowUp;
            Some(Gesture::DoubleTap)
        } else {
            self.prev = Prev::Down { at: now };
            Some(Gesture::Press)
        }
    }

    fn on_up(&mut self, now: Duration) -> Option<Gesture> {
        match self.prev {
            // Release ends a hold; a sub-threshold tap becomes a double-tap
            // candidate and emits nothing. `>=` matches the pipeline's
            // `held_for < TAP_THRESHOLD` tap test exactly.
            Prev::Down { at } => {
                if now.saturating_sub(at) >= self.hold {
                    self.prev = Prev::Idle;
                    Some(Gesture::Release)
                } else {
                    self.prev = Prev::PendingTap { up_at: now };
                    None
                }
            }
            // The double-tap's second `Up`: swallow it so it is not read as a
            // (sub-threshold) hold, then return to Idle.
            Prev::SwallowUp => {
                self.prev = Prev::Idle;
                None
            }
            // A stray `Up` with no open press (e.g. fn held across app launch).
            Prev::Idle | Prev::PendingTap { .. } => None,
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{ensure_monitor, input_monitoring_status, request_input_monitoring};

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::ptr::NonNull;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::Mutex;
    use std::time::Instant;

    use objc2_core_foundation::{kCFRunLoopCommonModes, CFMachPort, CFRunLoop};
    use objc2_core_graphics::{
        CGEvent, CGEventFlags, CGEventMask, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventTapProxy, CGEventType, CGPreflightListenEventAccess,
        CGRequestListenEventAccess,
    };
    use tauri::{AppHandle, Manager};

    use super::{FnEdge, Gesture, GestureState};
    use crate::pipeline::Job;
    use crate::shortcuts::is_fn_gesture;
    use crate::state::AppState;

    /// The single live `fn` monitor, or `None` until started. Held as a module
    /// static (like hud.rs's `MENU_OPEN`) so `shortcuts::apply` can reach it
    /// without a cfg-gated `AppState` field. Only `ensure_monitor` touches it,
    /// and only from `apply` (startup + each settings save), so there is no
    /// contention; the run-loop/consumer threads never lock it.
    static FN_MONITOR: Mutex<Option<FnMonitor>> = Mutex::new(None);

    /// Starts the `fn` monitor once if push-to-talk is an `fn` gesture and Input
    /// Monitoring is granted; returns whether the tap is now live. (Push-to-talk
    /// is the only `fn`-gesture trigger — hands-free is the tap-latch on that key
    /// plus an optional accelerator.) Called from `shortcuts::apply` on every
    /// settings save: a no-op once started, and a no-op (returning `false`) while
    /// ungranted or while push-to-talk is an accelerator, so `apply` keeps the
    /// accelerator fallback. Idempotent — the monitor is never torn down or
    /// double-started; routing follows live settings instead (see
    /// `route_gesture`).
    pub fn ensure_monitor(app: &AppHandle) -> bool {
        let mut guard = FN_MONITOR.lock().expect("fn monitor poisoned");
        if guard.is_some() {
            return true;
        }
        let settings = app.state::<AppState>().settings.get();
        if !is_fn_gesture(&settings.push_to_talk_hotkey) {
            // Nothing to observe yet (push-to-talk is an accelerator); a later
            // save that switches it to the `fn` gesture will start it.
            return false;
        }
        let routed = app.clone();
        match start_monitor(move |gesture| route_gesture(&routed, gesture)) {
            Some(monitor) => {
                *guard = Some(monitor);
                true
            }
            // Ungranted or tap creation failed: the caller keeps the fallback.
            None => false,
        }
    }

    /// Maps a recognized gesture onto the pipeline, driving ONLY push-to-talk
    /// (re-checking live settings so no teardown is needed when the trigger
    /// changes). Press starts and Release finishes; a quick tap of `fn` is a
    /// Press with no Release (its sub-threshold Up emits nothing), so the
    /// pipeline latches into hands-free until the next tap — that tap-latch IS
    /// hands-free on the `fn` key. DoubleTap is intentionally not routed: with
    /// the instant hold-Press, a second tap on the same key can't toggle
    /// hands-free, so a double-tap just latches like a single tap (its second Up
    /// is swallowed). Hands-free's optional separate accelerator is a Carbon
    /// hotkey in `shortcuts.rs`. Runs on the consumer thread, so it calls the
    /// pipeline directly.
    fn route_gesture(app: &AppHandle, gesture: Gesture) {
        let state = app.state::<AppState>();
        let settings = state.settings.get();
        let pipeline = state.pipeline.clone();
        if !is_fn_gesture(&settings.push_to_talk_hotkey) {
            return;
        }
        match gesture {
            Gesture::Press => pipeline.on_hotkey_pressed(Job::Dictation),
            Gesture::Release => pipeline.on_hotkey_released(Job::Dictation),
            // A double-tap latches via its first Press; nothing more to do.
            Gesture::DoubleTap => {}
        }
    }

    /// Tristate Input Monitoring status, mirrored as `inputMonitoring` on
    /// `PermissionsState` in `@velata/core` (string union, like `microphone`).
    /// `CGPreflightListenEventAccess` is a bool, so there is no "undetermined";
    /// "unknown" is reserved for the non-macOS stub.
    pub fn input_monitoring_status() -> &'static str {
        if CGPreflightListenEventAccess() {
            "granted"
        } else {
            "denied"
        }
    }

    /// Prompts for Input Monitoring (and adds Velata to the list). The grant
    /// usually takes effect only on the NEXT launch, so the caller keeps the
    /// accelerator fallback for this session.
    pub fn request_input_monitoring() {
        let granted = CGRequestListenEventAccess();
        log::info!("input monitoring request: granted={granted}");
    }

    /// Only the `fn` modifier matters; other modifier changes are ignored.
    const FN_BIT: u64 = CGEventFlags::MaskSecondaryFn.0;

    /// `flagsChanged` is the only event the `fn` key surfaces through.
    fn flags_changed_mask() -> CGEventMask {
        1u64 << (CGEventType::FlagsChanged.0 as u64)
    }

    /// State shared with the C tap callback through its `user_info` pointer. The
    /// callback runs only on the `velata-fn` run-loop thread (the tap serializes
    /// events), so `last_fn_down` needs no synchronization. `tap` is held so the
    /// callback can re-arm the tap if macOS disables it; it is set right after
    /// the tap is created.
    struct TapContext {
        last_fn_down: bool,
        sender: Sender<(FnEdge, Instant)>,
        tap: Option<objc2_core_foundation::CFRetained<CFMachPort>>,
    }

    /// The C-ABI tap callback. It cannot capture, so all state travels through
    /// `user_info`. Listen-only: it returns the event UNCHANGED and never blocks
    /// or panics — it only diffs the `fn` bit and forwards the edge.
    ///
    /// # Safety
    /// `user_info` must be the `*mut TapContext` passed to `tap_create`, valid
    /// for the lifetime of the tap (the consumer thread owns the box and only
    /// drops it after the run loop stops).
    unsafe extern "C-unwind" fn tap_callback(
        _proxy: CGEventTapProxy,
        event_type: CGEventType,
        event: NonNull<CGEvent>,
        user_info: *mut c_void,
    ) -> *mut CGEvent {
        // SAFETY: `user_info` is the context box pointer; the tap is the box's
        // sole reader and runs single-threaded on this run loop.
        let ctx = unsafe { &mut *(user_info as *mut TapContext) };

        // macOS disables a tap that is slow or interrupted; re-arm it and pass
        // the event through. Re-enabling needs the tap handle, kept in the
        // context.
        if event_type == CGEventType::TapDisabledByTimeout
            || event_type == CGEventType::TapDisabledByUserInput
        {
            if let Some(tap) = ctx.tap.as_deref() {
                CGEvent::tap_enable(tap, true);
            }
            return event.as_ptr();
        }

        if event_type == CGEventType::FlagsChanged {
            // SAFETY: a flagsChanged event carries valid flags.
            let flags = CGEvent::flags(Some(unsafe { event.as_ref() }));
            let fn_down = (flags.0 & FN_BIT) != 0;
            if fn_down != ctx.last_fn_down {
                ctx.last_fn_down = fn_down;
                let edge = if fn_down { FnEdge::Down } else { FnEdge::Up };
                // Stamp the time here so the gap measurement is immune to
                // consumer-thread scheduling latency. A full channel just drops
                // the edge — never block the event stream.
                let _ = ctx.sender.send((edge, Instant::now()));
            }
        }

        // Listen-only: never alter or swallow the event.
        event.as_ptr()
    }

    /// A running `fn` monitor: the `velata-fn` run-loop thread plus the consumer
    /// thread. Started once and kept alive for the process; the `FN_MONITOR`
    /// static guards against a second start on re-apply. The run-loop thread is
    /// detached (its handle is dropped) because `CFRunLoop::run` never returns;
    /// holding the consumer handle keeps the type honest about what is alive.
    struct FnMonitor {
        _consumer: std::thread::JoinHandle<()>,
    }

    /// Starts the listen-only `fn` tap and routes recognized gestures to
    /// `on_gesture`. Returns `None` when Input Monitoring is ungranted or the tap
    /// cannot be created (so the caller keeps the accelerator fallback). The
    /// closure runs on the dedicated `velata-fn-consumer` thread; calling the
    /// pipeline from it directly is correct (unlike the Carbon path, this is not
    /// the main thread) and keeps each gesture's effects in order — a Press is
    /// applied before its Release.
    fn start_monitor<F>(on_gesture: F) -> Option<FnMonitor>
    where
        F: Fn(Gesture) + Send + 'static,
    {
        // Non-prompting status check: if ungranted, do not even try to create
        // the tap (which would silently fail), so the fallback owns the session.
        if !CGPreflightListenEventAccess() {
            log::info!("input monitoring not granted; fn gestures use the accelerator fallback");
            return None;
        }

        let (tx, rx) = std::sync::mpsc::channel::<(FnEdge, Instant)>();
        // The run-loop thread reports whether the tap was actually created, so
        // the caller can fall back when creation fails despite the preflight.
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<bool>();

        // The CFRunLoop thread: create the tap, attach it to this thread's run
        // loop, and run forever. CoreGraphics/CoreFoundation only — must not be
        // the main thread (that is AppKit's) and must not be an async executor.
        std::thread::Builder::new()
            .name("velata-fn".into())
            .spawn(move || run_loop_thread(tx, ready_tx))
            .ok()?;

        // Wait for the tap to report success/failure before claiming the fn path.
        let created = ready_rx.recv().unwrap_or(false);
        if !created {
            log::warn!("could not create fn event tap; using the accelerator fallback");
            return None;
        }

        let consumer = std::thread::Builder::new()
            .name("velata-fn-consumer".into())
            .spawn(move || consume_edges(rx, on_gesture))
            .ok()?;

        Some(FnMonitor {
            _consumer: consumer,
        })
    }

    /// Builds the tap, attaches it to this thread's run loop, then runs the loop.
    /// `CFRunLoopRun` blocks forever, so this thread does nothing else.
    fn run_loop_thread(sender: Sender<(FnEdge, Instant)>, ready: Sender<bool>) {
        let ctx = Box::new(TapContext {
            last_fn_down: false,
            sender,
            tap: None,
        });
        let ctx_ptr = Box::into_raw(ctx);

        // SAFETY: `tap_callback` is a correct C-ABI callback and `ctx_ptr` is a
        // valid, leaked `TapContext` pointer that outlives the tap (the box is
        // intentionally never freed — the monitor lives for the process).
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::HIDEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                flags_changed_mask(),
                Some(tap_callback),
                ctx_ptr as *mut c_void,
            )
        };

        let Some(tap) = tap else {
            // SAFETY: nothing took ownership of the context, so reclaim and drop
            // the box rather than leaking it on the failure path.
            drop(unsafe { Box::from_raw(ctx_ptr) });
            let _ = ready.send(false);
            return;
        };

        // Hand the tap to the callback so it can re-arm itself on disable.
        // SAFETY: the consumer/run-loop is the only reader; nothing else touches
        // the context once the loop runs.
        unsafe { (*ctx_ptr).tap = Some(tap.clone()) };

        let source = CFMachPort::new_run_loop_source(None, Some(&tap), 0);
        let Some(source) = source else {
            drop(unsafe { Box::from_raw(ctx_ptr) });
            let _ = ready.send(false);
            return;
        };

        let Some(run_loop) = CFRunLoop::current() else {
            drop(unsafe { Box::from_raw(ctx_ptr) });
            let _ = ready.send(false);
            return;
        };
        // SAFETY: `kCFRunLoopCommonModes` is a CoreFoundation constant string,
        // valid for the program's lifetime.
        let mode = unsafe { kCFRunLoopCommonModes };
        run_loop.add_source(Some(&source), mode);

        let _ = ready.send(true);

        // Blocks this thread forever, dispatching tap callbacks. The context box
        // is leaked deliberately: it must stay alive as long as the tap fires.
        CFRunLoop::run();
    }

    /// Drains fn edges, runs them through the pure recognizer, and forwards each
    /// gesture to the consumer closure. Blocks on `recv`; no timer is needed
    /// because every gesture is decidable on the next edge.
    fn consume_edges<F>(rx: Receiver<(FnEdge, Instant)>, on_gesture: F)
    where
        F: Fn(Gesture),
    {
        let mut state = GestureState::default();
        // The first edge anchors the monotonic clock the recognizer sees.
        let origin = Instant::now();
        while let Ok((edge, at)) = rx.recv() {
            let elapsed = at.saturating_duration_since(origin);
            if let Some(gesture) = state.on_edge(edge, elapsed) {
                on_gesture(gesture);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn input_monitoring_status() -> &'static str {
    "unknown"
}

#[cfg(not(target_os = "macos"))]
pub fn request_input_monitoring() {}

/// No `fn` observation off macOS, so the tap is never live and the accelerator
/// fallback always owns the `fn`-gesture triggers.
#[cfg(not(target_os = "macos"))]
pub fn ensure_monitor(_app: &tauri::AppHandle) -> bool {
    false
}

/// Deep link into System Settings → Privacy & Security → Input Monitoring.
/// Same scheme as the Accessibility/Microphone links in `permissions.rs`.
pub const INPUT_MONITORING_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent";

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Drives a fresh recognizer through a sequence and collects every emitted
    /// gesture, so each test asserts the exact gesture stream.
    fn run(edges: &[(FnEdge, u64)]) -> Vec<Gesture> {
        let mut state = GestureState::default();
        edges
            .iter()
            .filter_map(|&(edge, at)| state.on_edge(edge, ms(at)))
            .collect()
    }

    #[test]
    fn hold_emits_press_then_release() {
        // Down, held past the threshold, then Up.
        assert_eq!(
            run(&[(FnEdge::Down, 0), (FnEdge::Up, 500)]),
            vec![Gesture::Press, Gesture::Release]
        );
    }

    #[test]
    fn single_tap_emits_press_only() {
        // A sub-threshold tap is a double-tap candidate, so its Up emits nothing.
        assert_eq!(
            run(&[(FnEdge::Down, 0), (FnEdge::Up, 50)]),
            vec![Gesture::Press]
        );
    }

    #[test]
    fn double_tap_emits_press_then_doubletap_and_swallows_second_up() {
        // tap, then a second Down within the gap → DoubleTap; the second Up is
        // swallowed (no trailing Release).
        assert_eq!(
            run(&[
                (FnEdge::Down, 0),
                (FnEdge::Up, 50),
                (FnEdge::Down, 100),
                (FnEdge::Up, 150),
            ]),
            vec![Gesture::Press, Gesture::DoubleTap]
        );
    }

    #[test]
    fn two_slow_taps_are_not_a_double_tap() {
        // The discriminating case: the second Down is past the gap, so it is a
        // fresh Press, not a DoubleTap.
        assert_eq!(
            run(&[(FnEdge::Down, 0), (FnEdge::Up, 50), (FnEdge::Down, 1000)]),
            vec![Gesture::Press, Gesture::Press]
        );
    }

    #[test]
    fn stray_up_with_no_press_is_ignored() {
        // fn released with no prior Down (e.g. held across app launch).
        assert_eq!(run(&[(FnEdge::Up, 0)]), vec![]);
    }

    #[test]
    fn double_tap_then_independent_hold() {
        // After a double-tap completes, a later hold is a normal Press+Release —
        // the swallowed Up must not poison the next press.
        assert_eq!(
            run(&[
                (FnEdge::Down, 0),
                (FnEdge::Up, 50),
                (FnEdge::Down, 100),
                (FnEdge::Up, 150),
                (FnEdge::Down, 2000),
                (FnEdge::Up, 2500),
            ]),
            vec![
                Gesture::Press,
                Gesture::DoubleTap,
                Gesture::Press,
                Gesture::Release
            ]
        );
    }

    #[test]
    fn hold_then_quick_repress_is_not_a_double_tap() {
        // A hold's Up resets to Idle, so a quick re-press is a fresh Press even
        // though the down-to-down delta is under the gap. Guards against
        // classifying on the down-to-down delta alone.
        assert_eq!(
            run(&[(FnEdge::Down, 0), (FnEdge::Up, 400), (FnEdge::Down, 500)]),
            vec![Gesture::Press, Gesture::Release, Gesture::Press]
        );
    }

    #[test]
    fn hold_threshold_boundary_is_a_release() {
        // Exactly HOLD_THRESHOLD counts as a hold (>=), mirroring the pipeline's
        // `held_for < TAP_THRESHOLD` tap test.
        assert_eq!(
            run(&[(FnEdge::Down, 0), (FnEdge::Up, 350)]),
            vec![Gesture::Press, Gesture::Release]
        );
    }

    #[test]
    fn just_under_hold_threshold_is_a_tap() {
        // 349ms < 350ms → tap (no Release), so it stays a double-tap candidate.
        assert_eq!(
            run(&[(FnEdge::Down, 0), (FnEdge::Up, 349)]),
            vec![Gesture::Press]
        );
    }

    #[test]
    fn double_tap_gap_boundary_is_exclusive() {
        // A second Down exactly at the gap is NOT a double-tap (`<` is strict),
        // so it is a fresh Press.
        assert_eq!(
            run(&[(FnEdge::Down, 0), (FnEdge::Up, 10), (FnEdge::Down, 410)]),
            vec![Gesture::Press, Gesture::Press]
        );
    }
}
