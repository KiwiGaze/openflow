import { describe, expect, it } from 'vitest';
import { nextTabId, SIDEBAR_TAB_IDS, type TabId } from './sidebarTabs.js';

describe('SIDEBAR_TAB_IDS', () => {
  it('lists Features then Settings in order', () => {
    expect([...SIDEBAR_TAB_IDS]).toEqual([
      'home',
      'insights',
      'dictionary',
      'snippets',
      'style',
      'transforms',
      'scratchpad',
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
    expect(nextTabId('home', 'ArrowDown')).toBe('insights');
    expect(nextTabId('home', 'ArrowRight')).toBe('insights');
  });

  it('retreats on ArrowUp/ArrowLeft', () => {
    expect(nextTabId('insights', 'ArrowUp')).toBe('home');
    expect(nextTabId('insights', 'ArrowLeft')).toBe('home');
  });

  it('wraps forward from the last tab to the first', () => {
    expect(nextTabId('about', 'ArrowDown')).toBe('home');
  });

  it('wraps backward from the first tab to the last', () => {
    expect(nextTabId('home', 'ArrowUp')).toBe('about');
  });

  it('crosses the section boundary in both directions', () => {
    expect(nextTabId('scratchpad', 'ArrowDown')).toBe('dictation');
    expect(nextTabId('dictation', 'ArrowUp')).toBe('scratchpad');
  });

  it('jumps to the first tab on Home and the last on End', () => {
    expect(nextTabId('models', 'Home')).toBe('home');
    expect(nextTabId('models', 'End')).toBe('about');
  });

  it('returns null for non-navigation keys and unknown tabs', () => {
    expect(nextTabId('home', 'Enter')).toBeNull();
    expect(nextTabId('nope' as TabId, 'ArrowDown')).toBeNull();
  });
});
