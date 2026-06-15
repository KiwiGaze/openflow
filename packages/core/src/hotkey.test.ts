import { describe, expect, it } from 'vitest';
import {
  acceleratorFromKeyboardEvent,
  effectiveAccelerator,
  formatAcceleratorMac,
  formatHotkey,
  HANDS_FREE_FALLBACK,
  isValidAccelerator,
  parseAccelerator,
  PUSH_TO_TALK_FALLBACK,
} from './hotkey.js';

describe('parseAccelerator', () => {
  it('accepts modifier + key combos', () => {
    expect(parseAccelerator('Alt+Space')).toEqual({ modifiers: ['Alt'], key: 'Space' });
    expect(parseAccelerator('Cmd+Shift+R')).toEqual({ modifiers: ['Cmd', 'Shift'], key: 'R' });
    expect(parseAccelerator('CommandOrControl+Backquote')).toEqual({
      modifiers: ['CommandOrControl'],
      key: 'Backquote',
    });
  });

  it('accepts bare F-keys but no other bare keys', () => {
    expect(isValidAccelerator('F5')).toBe(true);
    expect(isValidAccelerator('F24')).toBe(true);
    expect(isValidAccelerator('A')).toBe(false);
    expect(isValidAccelerator('Space')).toBe(false);
    expect(isValidAccelerator('9')).toBe(false);
  });

  it('rejects malformed input', () => {
    expect(isValidAccelerator('')).toBe(false);
    expect(isValidAccelerator('Alt+')).toBe(false);
    expect(isValidAccelerator('+Space')).toBe(false);
    expect(isValidAccelerator('Alt+Banana')).toBe(false);
    expect(isValidAccelerator('Banana+A')).toBe(false);
    expect(isValidAccelerator('Alt+Shift')).toBe(false);
    expect(isValidAccelerator('F25')).toBe(false);
  });

  it('is case-insensitive for tokens', () => {
    expect(isValidAccelerator('alt+space')).toBe(true);
    expect(isValidAccelerator('CMD+SHIFT+r')).toBe(true);
  });
});

describe('formatAcceleratorMac', () => {
  it('renders macOS glyphs', () => {
    expect(formatAcceleratorMac('Alt+Space')).toBe('⌥ Space');
    expect(formatAcceleratorMac('Cmd+Shift+R')).toBe('⌘ ⇧ R');
    expect(formatAcceleratorMac('Ctrl+Backquote')).toBe('⌃ `');
    expect(formatAcceleratorMac('F5')).toBe('F5');
  });

  it('returns input unchanged when unparseable', () => {
    expect(formatAcceleratorMac('garbage')).toBe('garbage');
  });
});

describe('acceleratorFromKeyboardEvent', () => {
  const base = { altKey: false, ctrlKey: false, metaKey: false, shiftKey: false };

  it('builds accelerators from physical key codes', () => {
    expect(acceleratorFromKeyboardEvent({ ...base, code: 'KeyR', metaKey: true })).toBe('Cmd+R');
    expect(acceleratorFromKeyboardEvent({ ...base, code: 'Space', altKey: true })).toBe(
      'Alt+Space',
    );
    expect(
      acceleratorFromKeyboardEvent({ ...base, code: 'Digit1', ctrlKey: true, shiftKey: true }),
    ).toBe('Ctrl+Shift+1');
    expect(acceleratorFromKeyboardEvent({ ...base, code: 'ArrowUp', altKey: true })).toBe('Alt+Up');
  });

  it('allows bare F-keys', () => {
    expect(acceleratorFromKeyboardEvent({ ...base, code: 'F6' })).toBe('F6');
  });

  it('rejects bare letters and unbindable codes', () => {
    expect(acceleratorFromKeyboardEvent({ ...base, code: 'KeyA' })).toBeNull();
    expect(acceleratorFromKeyboardEvent({ ...base, code: 'Escape', altKey: true })).toBeNull();
    expect(acceleratorFromKeyboardEvent({ ...base, code: 'AltLeft', altKey: true })).toBeNull();
  });
});

describe('formatHotkey', () => {
  it('labels the fn gesture defaults', () => {
    expect(formatHotkey({ kind: 'hold', key: 'fn' })).toBe('Hold fn');
    expect(formatHotkey({ kind: 'doubleTap', key: 'fn' })).toBe('Double-tap fn');
  });

  it('renders an accelerator with macOS glyphs, empty as "Not set"', () => {
    expect(formatHotkey({ kind: 'accelerator', key: 'Alt+O' })).toBe('⌥ O');
    expect(formatHotkey({ kind: 'accelerator', key: '' })).toBe('Not set');
    expect(formatHotkey({ kind: 'accelerator', key: '  ' })).toBe('Not set');
  });
});

describe('effectiveAccelerator', () => {
  it('uses the gesture fallback for an unobservable fn trigger', () => {
    expect(effectiveAccelerator({ kind: 'hold', key: 'fn' }, PUSH_TO_TALK_FALLBACK)).toBe(
      'Alt+Space',
    );
    expect(effectiveAccelerator({ kind: 'doubleTap', key: 'fn' }, HANDS_FREE_FALLBACK)).toBe(
      'Alt+Shift+Space',
    );
  });

  it('uses an accelerator trigger as-is, and reports an empty key as disabled', () => {
    expect(effectiveAccelerator({ kind: 'accelerator', key: 'Alt+O' }, PUSH_TO_TALK_FALLBACK)).toBe(
      'Alt+O',
    );
    expect(
      effectiveAccelerator({ kind: 'accelerator', key: '' }, PUSH_TO_TALK_FALLBACK),
    ).toBeNull();
  });
});
