import { useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Esc must close the window like Cmd+W, but never while the user is typing or
 * recording a hotkey: a text field swallows Esc for its own editing, and the
 * HotkeyRecorder (its active chip carries `.hotkey-recording`) uses Esc to
 * cancel — that cancel must win. Plain buttons (sidebar tabs) do not block Esc.
 */
function escapeShouldClose(active: Element | null): boolean {
  if (!active) return true;
  const tag = active.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return false;
  if (active instanceof HTMLElement && active.isContentEditable) return false;
  if (active.closest('.hotkey-recording')) return false;
  return true;
}

/**
 * Cmd+W and Esc close the current window. close() routes through the Rust
 * CloseRequested handler, which hides the window and keeps the app in the menu
 * bar — the same path as the red traffic-light. Shared by the App and Settings
 * windows, which both hide-on-close.
 */
export function useWindowClose(): void {
  useEffect(() => {
    const onKeyDown = (e: globalThis.KeyboardEvent): void => {
      if (e.metaKey && e.key.toLowerCase() === 'w') {
        e.preventDefault();
        void getCurrentWindow().close();
        return;
      }
      if (e.key === 'Escape' && !e.metaKey && escapeShouldClose(document.activeElement)) {
        e.preventDefault();
        void getCurrentWindow().close();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);
}
