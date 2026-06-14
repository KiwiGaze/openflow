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
  version: 5,
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
  inputDeviceName: null,
  insertMethod: 'paste',
  restoreClipboard: true,
  launchAtLogin: false,
  appearance: 'system',
  historyEnabled: false,
  historyRetentionDays: 0,
  appRules: [],
  autoCleanupLevel: 'ai',
  confirmedSttProfiles: [],
  snippets: [],
  transforms: [],
  polishRules: { concise: false, clarity: true, structure: false, tone: true },
  showInDock: false,
  scratchpadEnabled: false,
  tipsEnabled: true,
  tipsSeen: [],
  dictationCount: 0,
  lastTipShownAt: '',
  onboardingCompleted: true,
};

describe('eligibleTip', () => {
  it('shows nothing on a non-dictation page', () => {
    expect(eligibleTip('ai', { ...base, dictationCount: 4 }, '2026-06-11')).toBeNull();
  });

  it('respects tipsEnabled, the daily cap, and tipsSeen', () => {
    const s = { ...base, dictationCount: 4 };
    expect(eligibleTip('dictation', { ...s, tipsEnabled: false }, 'd')).toBeNull();
    expect(eligibleTip('dictation', { ...s, lastTipShownAt: 'd' }, 'd')).toBeNull();
    expect(eligibleTip('dictation', { ...s, tipsSeen: ['tip.ai'] }, 'd')).toBeNull();
  });

  it('shows tip.ai after 4 dictations when no AI profile is set', () => {
    const s = { ...base, dictationCount: 4 };
    expect(eligibleTip('dictation', s, 'd')?.id).toBe('tip.ai');
  });
});
