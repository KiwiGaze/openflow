/**
 * Sidebar information architecture: two labeled sections rendered as separate
 * tablists, but one continuous keyboard ring across both (Arrow keys wrap and
 * cross the section boundary; Home/End jump to the first/last tab overall).
 */

export type TabId =
  | 'home'
  | 'insights'
  | 'dictionary'
  | 'snippets'
  | 'style'
  | 'transforms'
  | 'scratchpad'
  | 'dictation'
  | 'modes'
  | 'models'
  | 'output'
  | 'general'
  | 'about';

export interface SidebarTab {
  id: TabId;
  label: string;
}

export interface SidebarSection {
  label: string;
  tabs: readonly SidebarTab[];
}

export const SIDEBAR_SECTIONS: readonly SidebarSection[] = [
  {
    label: 'Features',
    tabs: [
      { id: 'home', label: 'Home' },
      { id: 'insights', label: 'Insights' },
      { id: 'dictionary', label: 'Dictionary' },
      { id: 'snippets', label: 'Snippets' },
      { id: 'style', label: 'Style' },
      { id: 'transforms', label: 'Transforms' },
      { id: 'scratchpad', label: 'Scratchpad' },
    ],
  },
  {
    label: 'Settings',
    tabs: [
      { id: 'dictation', label: 'Dictation' },
      { id: 'modes', label: 'Modes' },
      { id: 'models', label: 'Models' },
      { id: 'output', label: 'Output' },
      { id: 'general', label: 'General' },
      { id: 'about', label: 'About' },
    ],
  },
];

/** The flat, ordered ring of tab ids — both sections, in display order. */
export const SIDEBAR_TAB_IDS: readonly TabId[] = SIDEBAR_SECTIONS.flatMap((s) =>
  s.tabs.map((t) => t.id),
);

/**
 * Given the current tab id and a navigation key, return the id to move to, or
 * null if the key is not a navigation key. ArrowDown/ArrowRight advance (wrap
 * past the last tab to the first); ArrowUp/ArrowLeft retreat (wrap past the
 * first to the last); Home/End jump to the ends. Section boundaries are
 * invisible to the ring — the order is purely SIDEBAR_TAB_IDS.
 */
export function nextTabId(current: TabId, key: string): TabId | null {
  const ids = SIDEBAR_TAB_IDS;
  const idx = ids.indexOf(current);
  if (idx === -1) return null;
  if (key === 'ArrowDown' || key === 'ArrowRight') return ids[(idx + 1) % ids.length] ?? null;
  if (key === 'ArrowUp' || key === 'ArrowLeft')
    return ids[(idx - 1 + ids.length) % ids.length] ?? null;
  if (key === 'Home') return ids[0] ?? null;
  if (key === 'End') return ids[ids.length - 1] ?? null;
  return null;
}
