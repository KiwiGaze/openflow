import { useEffect, useRef, useState, type JSX, type KeyboardEvent } from 'react';
import { useModels, useSettings } from './hooks.js';
import { Onboarding } from './Onboarding.js';
import { AboutTab } from './tabs/AboutTab.js';
import { DictionaryTab } from './tabs/DictionaryTab.js';
import { GeneralTab } from './tabs/GeneralTab.js';
import { ModesTab } from './tabs/ModesTab.js';
import { RefineTab } from './tabs/RefineTab.js';

const TABS = [
  { id: 'general', label: 'General' },
  { id: 'modes', label: 'Modes' },
  { id: 'dictionary', label: 'Dictionary' },
  { id: 'refine', label: 'Refine' },
  { id: 'about', label: 'About' },
] as const;

type TabId = (typeof TABS)[number]['id'];

export function App(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('general');
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  // Apply the theme override before content paints; `system` defers to the
  // OS via the CSS media query, so the dataset attribute is a no-op there.
  useEffect(() => {
    document.documentElement.dataset.theme = api?.settings.appearance ?? 'system';
  }, [api?.settings.appearance]);

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
              {t.label}
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
        {tab === 'general' && <GeneralTab api={api} modelsApi={modelsApi} />}
        {tab === 'modes' && <ModesTab api={api} />}
        {tab === 'dictionary' && <DictionaryTab api={api} />}
        {tab === 'refine' && <RefineTab api={api} />}
        {tab === 'about' && <AboutTab />}
      </main>
    </div>
  );
}
