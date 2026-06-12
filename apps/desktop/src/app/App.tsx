import { useEffect, useRef, useState, type JSX, type KeyboardEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { todayIso } from '@velata/core';
import { useModels, useSettings } from './hooks.js';
import { Onboarding } from './Onboarding.js';
import { eligibleTip } from './tips.js';
import { Callout } from './components/Callout.js';
import { TAB_ICONS } from './components/tabIcons.js';
import { nextTabId, SIDEBAR_SECTIONS, type TabId } from './sidebarTabs.js';
import { AboutTab } from './tabs/AboutTab.js';
import { DictationTab } from './tabs/DictationTab.js';
import { DictionaryTab } from './tabs/DictionaryTab.js';
import { GeneralTab } from './tabs/GeneralTab.js';
import { HomeTab } from './tabs/HomeTab.js';
import { InsightsTab } from './tabs/InsightsTab.js';
import { ModelsTab } from './tabs/ModelsTab.js';
import { ModesTab } from './tabs/ModesTab.js';
import { OutputTab } from './tabs/OutputTab.js';
import { ScratchpadTab } from './tabs/ScratchpadTab.js';
import { SnippetsTab } from './tabs/SnippetsTab.js';
import { StyleTab } from './tabs/StyleTab.js';
import { TransformsTab } from './tabs/TransformsTab.js';

export type { TabId };

/**
 * Esc must close the window like Cmd+W, but never while the user is typing or
 * recording a hotkey: a text field swallows Esc for its own editing, and the
 * HotkeyRecorder (its active chip carries `.hotkey-recording`) uses Esc to
 * cancel — that cancel must win. Plain buttons (sidebar tabs) do not block Esc.
 */
function escapeShouldClose(active: Element | null): boolean {
  if (!active) return true;
  const tag = active.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return false;
  if (active instanceof HTMLElement && active.isContentEditable) return false;
  if (active.closest('.hotkey-recording')) return false;
  return true;
}

export function App(): JSX.Element {
  const api = useSettings();
  const modelsApi = useModels();
  const [tab, setTab] = useState<TabId>('home');
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  // Apply the theme override before content paints; `system` defers to the
  // OS via the CSS media query, so the dataset attribute is a no-op there.
  useEffect(() => {
    document.documentElement.dataset.theme = api?.settings.appearance ?? 'system';
  }, [api?.settings.appearance]);

  // Cmd+W and Esc close the settings window (UX-14). close() routes through the
  // Rust CloseRequested handler, which hides it and drops back to Accessory —
  // the same path as the red traffic-light, so the app stays in the menu bar.
  useEffect(() => {
    const onKeyDown = (e: globalThis.KeyboardEvent): void => {
      if (e.metaKey && e.key.toLowerCase() === 'w') {
        e.preventDefault();
        void getCurrentWindow().close();
        return;
      }
      if (e.key === 'Escape' && !e.metaKey && escapeShouldClose(document.activeElement)) {
        e.preventDefault();
        void getCurrentWindow().close();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  // Arrow/Home/End move selection across both sections as one ring and follow
  // focus (the ARIA tabs pattern with roving tabindex).
  const onTabsKeyDown = (e: KeyboardEvent<HTMLDivElement>): void => {
    const next = nextTabId(tab, e.key);
    if (!next) return;
    e.preventDefault();
    setTab(next);
    tabRefs.current[next]?.focus();
  };

  if (!api) {
    return <div className="splash">Velata</div>;
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
      <nav className="sidebar">
        <div className="sidebar-brand">Velata</div>
        {SIDEBAR_SECTIONS.map((section) => (
          <div key={section.label}>
            <div className="sidebar-section-label" aria-hidden>
              {section.label}
            </div>
            <div
              role="tablist"
              aria-orientation="vertical"
              aria-label={section.label}
              className="sidebar-tabs"
              onKeyDown={onTabsKeyDown}
            >
              {section.tabs.map((t) => (
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
          </div>
        ))}
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
        {tab === 'home' && <HomeTab api={api} modelsApi={modelsApi} onNavigate={setTab} />}
        {tab === 'insights' && <InsightsTab api={api} />}
        {tab === 'dictionary' && <DictionaryTab api={api} />}
        {tab === 'snippets' && <SnippetsTab api={api} />}
        {tab === 'style' && <StyleTab api={api} />}
        {tab === 'transforms' && <TransformsTab api={api} />}
        {tab === 'scratchpad' && <ScratchpadTab />}
        {tab === 'dictation' && <DictationTab api={api} modelsApi={modelsApi} />}
        {tab === 'modes' && <ModesTab api={api} modelsApi={modelsApi} />}
        {tab === 'models' && <ModelsTab api={api} modelsApi={modelsApi} />}
        {tab === 'output' && <OutputTab api={api} />}
        {tab === 'general' && <GeneralTab api={api} />}
        {tab === 'about' && <AboutTab />}
      </main>
    </div>
  );
}
