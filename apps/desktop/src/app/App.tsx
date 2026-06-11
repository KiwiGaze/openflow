import { useEffect, useRef, useState, type JSX, type KeyboardEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { todayIso } from '@openflow/core';
import { useModels, useSettings } from './hooks.js';
import { Onboarding } from './Onboarding.js';
import { eligibleTip } from './tips.js';
import { Callout } from './components/Callout.js';
import { TAB_ICONS } from './components/tabIcons.js';
import { AboutTab } from './tabs/AboutTab.js';
import { DictationTab } from './tabs/DictationTab.js';
import { DictionaryTab } from './tabs/DictionaryTab.js';
import { GeneralTab } from './tabs/GeneralTab.js';
import { InsightsTab } from './tabs/InsightsTab.js';
import { ModelsTab } from './tabs/ModelsTab.js';
import { ModesTab } from './tabs/ModesTab.js';
import { OutputTab } from './tabs/OutputTab.js';
import { SnippetsTab } from './tabs/SnippetsTab.js';

const TABS = [
  { id: 'dictation', label: 'Dictation' },
  { id: 'modes', label: 'Modes' },
  { id: 'models', label: 'Models' },
  { id: 'output', label: 'Output' },
  { id: 'dictionary', label: 'Dictionary' },
  { id: 'snippets', label: 'Snippets' },
  { id: 'insights', label: 'Insights' },
  { id: 'general', label: 'General' },
  { id: 'about', label: 'About' },
] as const;

export type TabId = (typeof TABS)[number]['id'];

export function App(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('dictation');
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  // Apply the theme override before content paints; `system` defers to the
  // OS via the CSS media query, so the dataset attribute is a no-op there.
  useEffect(() => {
    document.documentElement.dataset.theme = api?.settings.appearance ?? 'system';
  }, [api?.settings.appearance]);

  // Cmd+W closes the settings window (UX-14). close() routes through the Rust
  // CloseRequested handler, which hides it and drops back to Accessory — the
  // same path as the red traffic-light, so the app stays in the menu bar.
  useEffect(() => {
    const onKeyDown = (e: globalThis.KeyboardEvent): void => {
      if (e.metaKey && e.key.toLowerCase() === 'w') {
        e.preventDefault();
        void getCurrentWindow().close();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  // Arrow/Home/End move selection within the tablist and follow focus (the
  // ARIA tabs pattern with roving tabindex).
  const onTabsKeyDown = (e: KeyboardEvent<HTMLDivElement>): void => {
    const idx = TABS.findIndex((t) => t.id === tab);
    let nextIdx: number | null = null;
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') nextIdx = (idx + 1) % TABS.length;
    else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft')
      nextIdx = (idx - 1 + TABS.length) % TABS.length;
    else if (e.key === 'Home') nextIdx = 0;
    else if (e.key === 'End') nextIdx = TABS.length - 1;
    const next = nextIdx === null ? undefined : TABS[nextIdx];
    if (!next) return;
    e.preventDefault();
    setTab(next.id);
    tabRefs.current[next.id]?.focus();
  };

  if (!api) {
    return <div className="splash">OpenFlow</div>;
  }

  if (!api.settings.onboardingCompleted) {
    return <Onboarding api={api} modelsApi={modelsApi} />;
  }

  const today = todayIso();
  const tip = eligibleTip(tab, api.settings, today);
  const dismissTip = (id: string): void => {
    void api.update({ tipsSeen: [...api.settings.tipsSeen, id], lastTipShownAt: today });
  };

  return (
    <div className="shell">
      <nav className="sidebar" aria-label="Settings">
        <div className="sidebar-brand">OpenFlow</div>
        <div
          role="tablist"
          aria-orientation="vertical"
          className="sidebar-tabs"
          onKeyDown={onTabsKeyDown}
        >
          {TABS.map((t) => (
            <button
              key={t.id}
              ref={(el) => {
                tabRefs.current[t.id] = el;
              }}
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={tab === t.id}
              aria-controls="settings-panel"
              tabIndex={tab === t.id ? 0 : -1}
              className={`sidebar-item ${tab === t.id ? 'sidebar-active' : ''}`}
              onClick={() => {
                setTab(t.id);
              }}
            >
              {TAB_ICONS[t.id]}
              <span>{t.label}</span>
            </button>
          ))}
        </div>
      </nav>
      <main className="content" role="tabpanel" id="settings-panel" aria-labelledby={`tab-${tab}`}>
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
        {tab === 'dictionary' && <DictionaryTab api={api} />}
        {tab === 'snippets' && <SnippetsTab api={api} />}
        {tab === 'insights' && <InsightsTab api={api} />}
        {tab === 'general' && <GeneralTab api={api} />}
        {tab === 'about' && <AboutTab />}
      </main>
    </div>
  );
}
