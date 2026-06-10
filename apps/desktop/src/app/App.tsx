import { useState, type JSX } from 'react';
import { useModels, useSettings } from './hooks.js';
import { Onboarding } from './Onboarding.js';
import { AboutTab } from './tabs/AboutTab.js';
import { DictionaryTab } from './tabs/DictionaryTab.js';
import { GeneralTab } from './tabs/GeneralTab.js';
import { ModesTab } from './tabs/ModesTab.js';
import { ProviderTab } from './tabs/ProviderTab.js';

const TABS = [
  { id: 'general', label: 'General' },
  { id: 'modes', label: 'Modes' },
  { id: 'dictionary', label: 'Dictionary' },
  { id: 'ai', label: 'AI Provider' },
  { id: 'about', label: 'About' },
] as const;

type TabId = (typeof TABS)[number]['id'];

export function App(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('general');

  if (!api) {
    return <div className="splash">OpenFlow</div>;
  }

  if (!api.settings.onboardingCompleted) {
    return <Onboarding api={api} modelsApi={modelsApi} />;
  }

  return (
    <div className="shell">
      <nav className="sidebar">
        <div className="sidebar-brand">OpenFlow</div>
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`sidebar-item ${tab === t.id ? 'sidebar-active' : ''}`}
            onClick={() => {
              setTab(t.id);
            }}
          >
            {t.label}
          </button>
        ))}
      </nav>
      <main className="content">
        {api.saveError && (
          <div className="error-banner">
            <span>{api.saveError}</span>
            <button className="btn btn-quiet" onClick={api.dismissError}>
              Dismiss
            </button>
          </div>
        )}
        {tab === 'general' && <GeneralTab api={api} modelsApi={modelsApi} />}
        {tab === 'modes' && <ModesTab api={api} />}
        {tab === 'dictionary' && <DictionaryTab api={api} />}
        {tab === 'ai' && <ProviderTab api={api} />}
        {tab === 'about' && <AboutTab />}
      </main>
    </div>
  );
}
