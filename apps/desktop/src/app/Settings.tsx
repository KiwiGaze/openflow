import { useEffect, useState, type JSX } from 'react';
import { todayIso } from '@velata/core';
import { useModels, useSettings } from './hooks.js';
import { events, ipc, subscribe } from './ipc.js';
import { useWindowClose } from './useWindowClose.js';
import { useThemeSync } from './useThemeSync.js';
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

/** Narrows a wire string to a Settings-window tab id, ignoring anything else. */
function isSettingsTab(id: string): id is TabId {
  return (SETTINGS_TAB_IDS as readonly string[]).includes(id);
}

/**
 * The Settings window: the configuration half of the former single window. It
 * shows the Settings-section tabs and a "‹ Velata" action that returns to the
 * App window. It shares the settings/model wiring, the close handler, and the
 * tips Callout with the App window, but has no onboarding gate — onboarding is
 * the App window's responsibility. It persists (hide-on-close), so a deep link
 * from the App window arrives as a `settings-navigate` event rather than a URL.
 */
export function Settings(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('dictation');

  useThemeSync(api?.settings.appearance);
  useWindowClose();

  // Follow a deep link from the App window (e.g. Home's Setup card → "models").
  useEffect(() => {
    return subscribe(
      events.onSettingsNavigate((id) => {
        if (isSettingsTab(id)) setTab(id);
      }),
    );
  }, []);

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
