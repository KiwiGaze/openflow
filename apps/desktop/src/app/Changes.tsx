import { useEffect, useMemo, useReducer, useState, type JSX } from 'react';
import { countChanges, diffWords, type InsertMethod } from '@velata/core';
import { events, ipc, subscribe } from './ipc.js';
import { initialChangesState, nextChangesState } from './changesState.js';

/**
 * The "see changes" overlay. Like the HUD the window is always present and the
 * content fades; unlike the HUD it takes clicks, so it lives in a non-activating
 * panel (see changes.rs) and reports its visibility to Rust so the frame is only
 * interactive while shown.
 */
export function Changes(): JSX.Element | null {
  const [state, dispatch] = useReducer(nextChangesState, initialChangesState);
  const [insertMethod, setInsertMethod] = useState<InsertMethod>('paste');

  useEffect(() => {
    void ipc.getSettings().then((s) => {
      setInsertMethod(s.insertMethod);
    });
    const cleanups = [
      subscribe(
        events.onChangesToggle((result) => {
          dispatch({ type: 'toggle', result });
        }),
      ),
      subscribe(
        events.onPipelineState((s) => {
          dispatch({ type: 'pipeline', status: s.status });
        }),
      ),
      subscribe(
        events.onSettingsChanged((s) => {
          setInsertMethod(s.insertMethod);
        }),
      ),
    ];
    return () => {
      cleanups.forEach((fn) => {
        fn();
      });
    };
  }, []);

  // Keep the OS frame click-through unless we are actually showing something.
  useEffect(() => {
    void ipc.setChangesInteractive(state.visible);
  }, [state.visible]);

  const { result, visible } = state;
  // The word diff allocates an O(n·m) table; pipeline events re-render the
  // open overlay, so compute it only when the result actually changes.
  const diff = useMemo(() => {
    if (!result) return null;
    const runs = diffWords(result.original, result.text);
    return { runs, changes: countChanges(runs) };
  }, [result]);
  if (!result || !diff) return null;

  const { runs, changes } = diff;

  return (
    <div
      className={`changes-backdrop ${visible ? 'changes-visible' : ''}`}
      onClick={() => {
        dispatch({ type: 'close' });
      }}
    >
      <div
        className="changes-card"
        onClick={(e) => {
          e.stopPropagation();
        }}
      >
        <div className="changes-head">
          <span className="changes-count">
            {changes === 0 ? 'No changes' : `${changes} ${changes === 1 ? 'Change' : 'Changes'}`}
          </span>
          {insertMethod === 'paste' && <span className="changes-undo">← ⌘Z to undo</span>}
        </div>
        <div className="changes-diff">
          {runs.map((run, i) =>
            run.op === 'equal' ? (
              <span key={i}>{run.text}</span>
            ) : run.op === 'delete' ? (
              <del key={i} className="diff-del">
                {run.text}
              </del>
            ) : (
              <ins key={i} className="diff-ins">
                {run.text}
              </ins>
            ),
          )}
        </div>
        <div className="changes-actions">
          <button
            type="button"
            className="btn btn-quiet"
            onClick={() => {
              void ipc.copyText(result.text);
            }}
          >
            Copy
          </button>
          <button
            type="button"
            className="btn btn-quiet"
            onClick={() => {
              dispatch({ type: 'close' });
            }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
