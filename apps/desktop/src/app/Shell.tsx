import { useRef, type JSX, type KeyboardEvent, type ReactNode } from 'react';
import { TAB_ICONS } from './components/tabIcons.js';
import { nextTabId, type SidebarSection, type TabId } from './sidebarTabs.js';

/** The bottom-of-sidebar action that switches to the other window. */
export interface SidebarAction {
  label: string;
  /** Inline glyph rendered before the label (e.g. a gear or chevron). */
  glyph: string;
  onClick: () => void;
}

/**
 * The sidebar + content scaffold shared by the App and Settings windows. Each
 * window passes its own sections, its current tab, and a setter; the shell owns
 * the roving-tabindex keyboard ring (closed within `ring`) and the bottom
 * action button. Tab panels are passed as `children` so each window keeps its
 * own content, error banner, tips, and gating.
 */
export function Shell({
  sections,
  ring,
  tab,
  onTabChange,
  action,
  children,
}: {
  sections: readonly SidebarSection[];
  /** This window's tab ring; navigation wraps within it and never leaves it. */
  ring: readonly TabId[];
  tab: TabId;
  onTabChange: (tab: TabId) => void;
  action: SidebarAction;
  children: ReactNode;
}): JSX.Element {
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  const onTabsKeyDown = (e: KeyboardEvent<HTMLDivElement>): void => {
    const next = nextTabId(ring, tab, e.key);
    if (!next) return;
    e.preventDefault();
    onTabChange(next);
    tabRefs.current[next]?.focus();
  };

  return (
    <div className="shell">
      <nav className="sidebar">
        <div className="sidebar-brand">Velata</div>
        {sections.map((section) => (
          <div key={section.ariaLabel}>
            {section.label && (
              <div className="sidebar-section-label" aria-hidden>
                {section.label}
              </div>
            )}
            <div
              role="tablist"
              aria-orientation="vertical"
              aria-label={section.ariaLabel}
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
                    onTabChange(t.id);
                  }}
                >
                  {TAB_ICONS[t.id]}
                  <span>{t.label}</span>
                </button>
              ))}
            </div>
          </div>
        ))}
        {/* An action, not a tab — kept out of the tablist so it stays clear of
            the roving ring while remaining reachable with Tab. */}
        <button type="button" className="sidebar-item sidebar-bottom" onClick={action.onClick}>
          <span className="sidebar-bottom-glyph" aria-hidden>
            {action.glyph}
          </span>
          <span>{action.label}</span>
        </button>
      </nav>
      <main className="content" role="tabpanel" id="settings-panel" aria-labelledby={`tab-${tab}`}>
        {children}
      </main>
    </div>
  );
}
