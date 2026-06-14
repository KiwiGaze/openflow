import type { Settings } from '@velata/core';
import type { TabId } from './App.js';

export interface Tip {
  id: string;
  /** Sidebar page this settings-card tip lives on. */
  page: string;
  copy: string;
  actionLabel: string;
  /** Tab to switch to when the action is taken. */
  actionTab: TabId;
  predicate: (s: Settings) => boolean;
}

/**
 * Settings-card tips (05 §2.4). Triggered from local settings state only —
 * never usage analytics, no network, no event log. HUD tips are P2-3.
 */
const TIPS: Tip[] = [
  {
    id: 'tip.ai',
    page: 'dictation',
    copy: 'Want sharper cleanup? Add a local Ollama model or your own API key and Velata will polish transcripts with AI.',
    actionLabel: 'Set up AI',
    actionTab: 'ai',
    predicate: (s) => s.dictationCount >= 4 && s.activeLlmProfileId === '',
  },
];

/**
 * The one eligible settings-card tip for a page, or null. At most one tip per
 * day (`lastTipShownAt`) and never a tip already in `tipsSeen`; the global
 * `tipsEnabled` switch kills all of them.
 */
export function eligibleTip(page: string, settings: Settings, today: string): Tip | null {
  if (!settings.tipsEnabled || settings.lastTipShownAt === today) return null;
  return (
    TIPS.find(
      (t) => t.page === page && !settings.tipsSeen.includes(t.id) && t.predicate(settings),
    ) ?? null
  );
}
