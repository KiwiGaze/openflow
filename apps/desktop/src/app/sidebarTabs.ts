/**
 * Sidebar information architecture. The app is split into two windows, each
 * with its own sidebar: the App window shows the Features tabs, the Settings
 * window shows the Settings tabs. Each window's tabs form one self-contained
 * keyboard ring — Arrow keys wrap within that window and never cross into the
 * other window's tabs; Home/End jump to the first/last tab of that window.
 */

export type TabId =
  | 'home'
  | 'library'
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

/** Sections shown in the App window (the Features tabs). */
export const APP_SECTIONS: readonly SidebarSection[] = [
  {
    label: 'Features',
    tabs: [
      { id: 'home', label: 'Home' },
      { id: 'library', label: 'Library' },
      { id: 'transforms', label: 'Transform' },
      { id: 'scratchpad', label: 'Scratchpad' },
    ],
  },
];

/** Sections shown in the Settings window (the Settings tabs). */
export const SETTINGS_SECTIONS: readonly SidebarSection[] = [
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

const flatIds = (sections: readonly SidebarSection[]): readonly TabId[] =>
  sections.flatMap((s) => s.tabs.map((t) => t.id));

/** The App window's keyboard ring, in display order. */
export const APP_TAB_IDS: readonly TabId[] = flatIds(APP_SECTIONS);

/** The Settings window's keyboard ring, in display order. */
export const SETTINGS_TAB_IDS: readonly TabId[] = flatIds(SETTINGS_SECTIONS);

/**
 * Given a window's tab ring, the current tab id, and a navigation key, return
 * the id to move to, or null if the key is not a navigation key or `current` is
 * not in this ring. ArrowDown/ArrowRight advance (wrap past the last tab to the
 * first); ArrowUp/ArrowLeft retreat (wrap past the first to the last); Home/End
 * jump to the ends. The ring is closed: navigation never leaves it.
 */
export function nextTabId(ids: readonly TabId[], current: TabId, key: string): TabId | null {
  const idx = ids.indexOf(current);
  if (idx === -1) return null;
  if (key === 'ArrowDown' || key === 'ArrowRight') return ids[(idx + 1) % ids.length] ?? null;
  if (key === 'ArrowUp' || key === 'ArrowLeft')
    return ids[(idx - 1 + ids.length) % ids.length] ?? null;
  if (key === 'Home') return ids[0] ?? null;
  if (key === 'End') return ids[ids.length - 1] ?? null;
  return null;
}
