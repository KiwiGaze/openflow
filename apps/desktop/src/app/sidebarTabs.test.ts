import { describe, expect, it } from 'vitest';
import { APP_TAB_IDS, nextTabId, SETTINGS_TAB_IDS, type TabId } from './sidebarTabs.js';

describe('tab rings', () => {
  it('lists the App (Features) tabs in order', () => {
    expect([...APP_TAB_IDS]).toEqual([
      'home',
      'insights',
      'dictionary',
      'snippets',
      'style',
      'transforms',
      'scratchpad',
    ]);
  });

  it('lists the Settings tabs in order', () => {
    expect([...SETTINGS_TAB_IDS]).toEqual([
      'dictation',
      'modes',
      'models',
      'output',
      'general',
      'about',
    ]);
  });
});

describe('nextTabId', () => {
  it('advances on ArrowDown/ArrowRight', () => {
    expect(nextTabId(APP_TAB_IDS, 'home', 'ArrowDown')).toBe('insights');
    expect(nextTabId(APP_TAB_IDS, 'home', 'ArrowRight')).toBe('insights');
  });

  it('retreats on ArrowUp/ArrowLeft', () => {
    expect(nextTabId(APP_TAB_IDS, 'insights', 'ArrowUp')).toBe('home');
    expect(nextTabId(APP_TAB_IDS, 'insights', 'ArrowLeft')).toBe('home');
  });

  it('wraps within each ring and never crosses to the other window', () => {
    // App ring: last (scratchpad) wraps to its own first (home), not into Settings.
    expect(nextTabId(APP_TAB_IDS, 'scratchpad', 'ArrowDown')).toBe('home');
    expect(nextTabId(APP_TAB_IDS, 'home', 'ArrowUp')).toBe('scratchpad');
    // Settings ring: last (about) wraps to its own first (dictation), not into the App.
    expect(nextTabId(SETTINGS_TAB_IDS, 'about', 'ArrowDown')).toBe('dictation');
    expect(nextTabId(SETTINGS_TAB_IDS, 'dictation', 'ArrowUp')).toBe('about');
  });

  it('jumps to the first/last tab of the given ring on Home/End', () => {
    expect(nextTabId(APP_TAB_IDS, 'style', 'Home')).toBe('home');
    expect(nextTabId(APP_TAB_IDS, 'style', 'End')).toBe('scratchpad');
    expect(nextTabId(SETTINGS_TAB_IDS, 'models', 'Home')).toBe('dictation');
    expect(nextTabId(SETTINGS_TAB_IDS, 'models', 'End')).toBe('about');
  });

  it('returns null for non-navigation keys and ids outside the given ring', () => {
    expect(nextTabId(APP_TAB_IDS, 'home', 'Enter')).toBeNull();
    expect(nextTabId(APP_TAB_IDS, 'nope' as TabId, 'ArrowDown')).toBeNull();
    // A Settings tab is not in the App ring (and vice versa).
    expect(nextTabId(APP_TAB_IDS, 'dictation', 'ArrowDown')).toBeNull();
    expect(nextTabId(SETTINGS_TAB_IDS, 'home', 'ArrowDown')).toBeNull();
  });
});
