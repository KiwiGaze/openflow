import { describe, expect, it } from 'vitest';
import {
  acceleratorFromKeyboardEvent,
  formatAcceleratorMac,
  isValidAccelerator,
  parseAccelerator,
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
