import { describe, expect, it } from 'vitest';
import type { Mode, Settings } from '@velata/core';
import { eligibleTip } from './tips.js';

const standardMode: Mode = {
  id: 'standard',
  name: 'Standard',
  builtIn: true,
  usesLlm: true,
  transforms: false,
  prompt: '',
  aiProfileId: null,
  sttModelId: null,
  language: null,
  hotkey: null,
};

const base: Settings = {
  version: 4,
  dictationHotkey: 'Alt+Space',
  dictationHotkeyBehavior: 'hold',
  polishHotkey: 'Alt+Shift+P',
  changeOverlayHotkey: 'Alt+O',
  polishAfterDictation: true,
  activeLlmProfileId: '',
  activeModeId: 'standard',
  modes: [standardMode],
  dictionary: [],
  sttModelId: 'base.en',
  language: 'auto',
  insertMethod: 'paste',
  restoreClipboard: true,
  launchAtLogin: false,
  appearance: 'system',
  historyEnabled: false,
  appRules: [],
  confirmedSttProfiles: [],
  snippets: [],
  transforms: [],
  showInDock: false,
  tipsEnabled: true,
  tipsSeen: [],
  dictationCount: 0,
  lastTipShownAt: '',
  onboardingCompleted: true,
};

describe('eligibleTip', () => {
  it('shows tip.modes after 3 dictations with only built-in modes', () => {
    expect(eligibleTip('dictation', { ...base, dictationCount: 3 }, '2026-06-11')?.id).toBe(
      'tip.modes',
    );
  });

  it('shows nothing on a non-dictation page', () => {
    expect(eligibleTip('models', { ...base, dictationCount: 4 }, '2026-06-11')).toBeNull();
  });

  it('respects tipsEnabled, the daily cap, and tipsSeen', () => {
    const s = { ...base, dictationCount: 3 };
    expect(eligibleTip('dictation', { ...s, tipsEnabled: false }, 'd')).toBeNull();
    expect(eligibleTip('dictation', { ...s, lastTipShownAt: 'd' }, 'd')).toBeNull();
    expect(eligibleTip('dictation', { ...s, tipsSeen: ['tip.modes'] }, 'd')).toBeNull();
  });

  it('shows tip.ai once a custom mode exists but no AI profile is set', () => {
    const custom: Mode = { ...standardMode, id: 'custom', builtIn: false };
    const s = { ...base, dictationCount: 4, modes: [standardMode, custom] };
    expect(eligibleTip('dictation', s, 'd')?.id).toBe('tip.ai');
  });
});
