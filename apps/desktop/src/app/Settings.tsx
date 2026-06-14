import { useEffect, useState, type JSX } from 'react';
import { todayIso } from '@velata/core';
import { useModels, useSettings } from './hooks.js';
import { ipc } from './ipc.js';
import { useWindowClose } from './useWindowClose.js';
import { eligibleTip } from './tips.js';
import { Callout } from './components/Callout.js';
import { Shell } from './Shell.js';
import { SETTINGS_SECTIONS, SETTINGS_TAB_IDS, type TabId } from './sidebarTabs.js';
import { AboutTab } from './tabs/AboutTab.js';
import { DictationTab } from './tabs/DictationTab.js';
import { GeneralTab } from './tabs/GeneralTab.js';
import { ModelsTab } from './tabs/ModelsTab.js';
import { ModesTab } from './tabs/ModesTab.js';
import { OutputTab } from './tabs/OutputTab.js';

/**
 * The Settings window: the configuration half of the former single window. It
 * shows the Settings-section tabs and a "‹ Velata" action that returns to the
 * App window. It shares the settings/model wiring, the close handler, and the
 * tips Callout with the App window, but has no onboarding gate — onboarding is
 * the App window's responsibility.
 */
export function Settings(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('dictation');

  useEffect(() => {
    document.documentElement.dataset.theme = api?.settings.appearance ?? 'system';
  }, [api?.settings.appearance]);

  useWindowClose();

  if (!api) {
    return <div className="splash">Velata</div>;
  }

  const today = todayIso();
  const tip = eligibleTip(tab, api.settings, today);
  const dismissTip = (id: string): void => {
    void api.update({ tipsSeen: [...api.settings.tipsSeen, id], lastTipShownAt: today });
  };

  return (
    <Shell
      sections={SETTINGS_SECTIONS}
      ring={SETTINGS_TAB_IDS}
      tab={tab}
      onTabChange={setTab}
      action={{ label: 'Velata', glyph: '‹', onClick: () => void ipc.openMainWindow() }}
    >
      {api.saveError && (
        <div className="error-banner" role="alert">
          <span>{api.saveError}</span>
          <button className="btn btn-quiet" onClick={api.dismissError}>
            Dismiss
          </button>
        </div>
      )}
      {tip && (
        <Callout
          variant="info"
          action={{
            label: tip.actionLabel,
            onClick: () => {
              setTab(tip.actionTab);
              dismissTip(tip.id);
            },
          }}
          onDismiss={() => {
            dismissTip(tip.id);
          }}
        >
          {tip.copy}
        </Callout>
      )}
      {tab === 'dictation' && <DictationTab api={api} modelsApi={modelsApi} />}
      {tab === 'modes' && <ModesTab api={api} modelsApi={modelsApi} />}
      {tab === 'models' && <ModelsTab api={api} modelsApi={modelsApi} />}
      {tab === 'output' && <OutputTab api={api} />}
      {tab === 'general' && <GeneralTab api={api} />}
      {tab === 'about' && <AboutTab />}
    </Shell>
  );
}
