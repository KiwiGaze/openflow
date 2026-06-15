import { useState, type JSX } from 'react';
import { useModels, useSettings } from './hooks.js';
import { ipc } from './ipc.js';
import { useWindowClose } from './useWindowClose.js';
import { useThemeSync } from './useThemeSync.js';
import { Onboarding } from './Onboarding.js';
import { Shell } from './Shell.js';
import { APP_SECTIONS, APP_TAB_IDS, type TabId } from './sidebarTabs.js';
import { HomeTab } from './tabs/HomeTab.js';
import { LibraryTab } from './tabs/LibraryTab.js';
import { ScratchpadTab } from './tabs/ScratchpadTab.js';
import { TransformsTab } from './tabs/TransformsTab.js';

export type { TabId };

export function App(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('home');

  useThemeSync(api?.settings.appearance);
  useWindowClose();

  if (!api) {
    return <div className="splash">Velata</div>;
  }

  if (!api.settings.onboardingCompleted) {
    return <Onboarding api={api} modelsApi={modelsApi} />;
  }

  // Deep links from Home may target a Settings-section tab, which this window
  // does not render; open the Settings window on that tab instead.
  const navigate = (next: TabId): void => {
    if (APP_TAB_IDS.includes(next)) {
      setTab(next);
    } else {
      void ipc.openSettingsWindow(next);
    }
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
      {tab === 'home' && <HomeTab api={api} modelsApi={modelsApi} onNavigate={navigate} />}
      {tab === 'library' && <LibraryTab api={api} />}
      {tab === 'transforms' && <TransformsTab api={api} />}
      {tab === 'scratchpad' && <ScratchpadTab api={api} />}
    </Shell>
  );
}
