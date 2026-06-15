import { useCallback, useEffect, useState, type JSX } from 'react';
import type { PipelineState, Prompt, Settings } from '@velata/core';
import { events, ipc, subscribe } from './ipc.js';
import { barScales, hudGlyph, hudLabel, hudVisible } from './hudState.js';

/**
 * Prompts in menu order: the built-in Polish first, then user prompts in their
 * stored order. The post-dictation transform stores the chosen prompt's real id
 * (Polish included); `null` means Off — unlike the Scratchpad, where null is
 * Polish. So Polish is a real radio here, not the empty selection.
 */
function orderedPrompts(prompts: Prompt[]): Prompt[] {
  const builtIns = prompts.filter((p) => p.builtIn);
  const custom = prompts.filter((p) => !p.builtIn);
  return [...builtIns, ...custom];
}

/**
 * The always-running overlay. The window itself never hides (see hud.rs);
 * content fades in and out with pipeline state instead. The circle is the one
 * interactive element: clicking it opens an upward radio menu that picks the
 * post-dictation transform. A Rust cursor poll makes only the circle (or the
 * whole frame while the menu is open) clickable, so the rest stays click-through
 * and never steals focus from the app being dictated into.
 */
export function Hud(): JSX.Element {
  const [state, setState] = useState<PipelineState>({
    status: 'idle',
    job: null,
    message: null,
    hudTip: null,
  });
  const [level, setLevel] = useState(0);
  const [prompts, setPrompts] = useState<Prompt[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    const cleanups = [
      subscribe(events.onPipelineState(setState)),
      subscribe(events.onAudioLevel(setLevel)),
      subscribe(
        events.onSettingsChanged((settings: Settings) => {
          setPrompts(settings.prompts);
          setActiveId(settings.postDictationTransformId);
        }),
      ),
    ];
    void ipc.getSettings().then((settings) => {
      setPrompts(settings.prompts);
      setActiveId(settings.postDictationTransformId);
    });
    return () => {
      cleanups.forEach((fn) => {
        fn();
      });
    };
  }, []);

  // Tell Rust whether the dropdown is open so its poll forces the whole window
  // interactive while open (the menu sits outside the circle's hit rect).
  const closeMenu = useCallback((): void => {
    setMenuOpen(false);
    void ipc.setHudMenuOpen(false);
  }, []);

  const toggleMenu = useCallback((): void => {
    setMenuOpen((open) => {
      const next = !open;
      void ipc.setHudMenuOpen(next);
      return next;
    });
  }, []);

  const select = useCallback(
    (id: string | null): void => {
      setActiveId(id);
      void ipc.setPostDictationTransform(id);
      closeMenu();
    },
    [closeMenu],
  );

  const visible = hudVisible(state);
  const recording = state.status === 'recording';
  const busy =
    state.status === 'transcribing' || state.status === 'polishing' || state.status === 'inserting';
  const glyph = hudGlyph(state);
  const transformOn = activeId !== null;

  return (
    <div
      className="hud-root"
      onClick={() => {
        // Click-away dismiss. Harmless at idle: the root is click-through then,
        // so this only fires when the menu has made the frame interactive.
        if (menuOpen) closeMenu();
      }}
    >
      <div className={`hud-pill ${visible ? 'hud-visible' : ''} hud-${state.status}`}>
        {recording && (
          <div className="hud-bars" aria-hidden>
            {barScales(level).map((scale, i) => (
              <span key={i} style={{ transform: `scaleY(${scale})` }} />
            ))}
          </div>
        )}
        {busy && <span className="hud-spinner" aria-hidden />}
        {glyph && (
          <span
            className={`hud-glyph ${state.status === 'inserted' ? 'hud-glyph-ok' : ''}`}
            aria-hidden
          >
            {glyph}
          </span>
        )}
        <span className="hud-textcol">
          <span className="hud-label" aria-live="polite" aria-atomic="true">
            {hudLabel(state)}
          </span>
          {state.hudTip && <span className="hud-tip">{state.hudTip}</span>}
        </span>
      </div>
      {/*
       * Post-dictation transform control. A sibling of the pill, not a child, so
       * it stays visible at idle when the pill fades out. Pinned bottom-right
       * because the Rust hit-test anchors to that fixed corner (see hud.rs).
       */}
      {menuOpen && (
        <ul className="hud-menu" role="menu" aria-label="Run after dictation">
          <li role="none">
            <button
              role="menuitemradio"
              aria-checked={activeId === null}
              className={`hud-menu-item ${activeId === null ? 'hud-menu-checked' : ''}`}
              onClick={(e) => {
                e.stopPropagation();
                select(null);
              }}
            >
              Off — insert as spoken
            </button>
          </li>
          {orderedPrompts(prompts).map((p) => (
            <li role="none" key={p.id}>
              <button
                role="menuitemradio"
                aria-checked={activeId === p.id}
                className={`hud-menu-item ${activeId === p.id ? 'hud-menu-checked' : ''}`}
                onClick={(e) => {
                  e.stopPropagation();
                  select(p.id);
                }}
              >
                {p.name}
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        type="button"
        className="hud-circle"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        aria-label="Run after dictation"
        onClick={(e) => {
          e.stopPropagation();
          toggleMenu();
        }}
        onKeyDown={(e) => {
          // Inert at runtime: the panel never takes key focus, so Escape goes to
          // the app being dictated into. Kept for the preview harness and as a
          // no-cost affordance if focus ever reaches the button.
          if (e.key === 'Escape' && menuOpen) closeMenu();
        }}
      >
        {transformOn ? '◉' : '◯'}
      </button>
    </div>
  );
}
