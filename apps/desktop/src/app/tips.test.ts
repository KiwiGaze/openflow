import { describe, expect, it } from 'vitest';
import type { Settings } from '@velata/core';
import { eligibleTip } from './tips.js';

const base: Settings = {
  version: 6,
  dictationHotkey: 'Alt+Space',
  dictationHotkeyBehavior: 'hold',
  changeOverlayHotkey: 'Alt+O',
  activeLlmProfileId: '',
  dictionary: [],
  snippets: [],
  prompts: [],
  postDictationTransformId: null,
  sttModelId: 'base.en',
  language: 'auto',
  inputDeviceName: null,
  launchAtLogin: false,
  appearance: 'system',
  historyEnabled: false,
  historyRetentionDays: 0,
  confirmedSttProfiles: [],
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
