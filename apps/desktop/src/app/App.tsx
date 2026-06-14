import { useEffect, useState, type JSX } from 'react';
import { todayIso } from '@velata/core';
import { useModels, useSettings } from './hooks.js';
import { ipc } from './ipc.js';
import { useWindowClose } from './useWindowClose.js';
import { Onboarding } from './Onboarding.js';
import { eligibleTip } from './tips.js';
import { Callout } from './components/Callout.js';
import { Shell } from './Shell.js';
import { APP_SECTIONS, APP_TAB_IDS, type TabId } from './sidebarTabs.js';
import { DictionaryTab } from './tabs/DictionaryTab.js';
import { HomeTab } from './tabs/HomeTab.js';
import { InsightsTab } from './tabs/InsightsTab.js';
import { ScratchpadTab } from './tabs/ScratchpadTab.js';
import { SnippetsTab } from './tabs/SnippetsTab.js';
import { StyleTab } from './tabs/StyleTab.js';
import { TransformsTab } from './tabs/TransformsTab.js';

export type { TabId };

export function App(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('home');

  // Apply the theme override before content paints; `system` defers to the
  // OS via the CSS media query, so the dataset attribute is a no-op there.
  useEffect(() => {
    document.documentElement.dataset.theme = api?.settings.appearance ?? 'system';
  }, [api?.settings.appearance]);

  useWindowClose();

  if (!api) {
    return <div className="splash">Velata</div>;
  }

  if (!api.settings.onboardingCompleted) {
    return <Onboarding api={api} modelsApi={modelsApi} />;
  }

  // Deep links from Home/tips may target a Settings-section tab, which this
  // window does not render; route those to the Settings window instead.
  const navigate = (next: TabId): void => {
    if (APP_TAB_IDS.includes(next)) {
      setTab(next);
    } else {
      void ipc.openSettingsWindow();
    }
  };

  const today = todayIso();
  const tip = eligibleTip(tab, api.settings, today);
  const dismissTip = (id: string): void => {
    void api.update({ tipsSeen: [...api.settings.tipsSeen, id], lastTipShownAt: today });
  };

  return (
    <Shell
      sections={APP_SECTIONS}
      ring={APP_TAB_IDS}
      tab={tab}
      onTabChange={setTab}
      action={{ label: 'Settings', glyph: '⚙', onClick: () => void ipc.openSettingsWindow() }}
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
              navigate(tip.actionTab);
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
      {tab === 'home' && <HomeTab api={api} modelsApi={modelsApi} onNavigate={navigate} />}
      {tab === 'insights' && <InsightsTab api={api} />}
      {tab === 'dictionary' && <DictionaryTab api={api} />}
      {tab === 'snippets' && <SnippetsTab api={api} />}
      {tab === 'style' && <StyleTab api={api} />}
      {tab === 'transforms' && <TransformsTab api={api} />}
      {tab === 'scratchpad' && <ScratchpadTab api={api} />}
    </Shell>
  );
}
