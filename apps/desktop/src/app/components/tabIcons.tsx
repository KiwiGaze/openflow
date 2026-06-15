import type { JSX } from 'react';
import type { TabId } from '../sidebarTabs.js';

/**
 * One inline SVG per sidebar page (09 §5.7). Decorative (aria-hidden); the
 * label is the accessible name. Stroke is currentColor so the icon flips to
 * white on the active row. No icon library (00 §8.5).
 */
function Icon({ children }: { children: JSX.Element }): JSX.Element {
  return (
    <svg
      className="sidebar-icon"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.4}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      {children}
    </svg>
  );
}

export const TAB_ICONS: Record<TabId, JSX.Element> = {
  // house
  home: (
    <Icon>
      <>
        <path d="M2.5 7.5 8 3l5.5 4.5" />
        <path d="M3.5 6.8V13a.5.5 0 0 0 .5.5h8a.5.5 0 0 0 .5-.5V6.8" />
        <path d="M6.5 13.5V9.5h3v4" />
      </>
    </Icon>
  ),
  // wand with sparks
  transforms: (
    <Icon>
      <>
        <path d="M4 12 11 5" />
        <path d="M10 3.5 11 4.5M12.5 6l1 1M3 6.5l.7.7M5.5 3l.5.7M5.5 8l.7.5" />
      </>
    </Icon>
  ),
  // note with pencil
  scratchpad: (
    <Icon>
      <>
        <path d="M11 8.5V12a.5.5 0 0 1-.5.5h-7A.5.5 0 0 1 3 12V4.5A.5.5 0 0 1 3.5 4H7" />
        <path d="M9 3.5 12.5 7 8 11.5H5.5V9z" />
      </>
    </Icon>
  ),
  // microphone
  dictation: (
    <Icon>
      <>
        <rect x="6" y="1.5" width="4" height="7.5" rx="2" />
        <path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.5M5.5 14.5h5" />
      </>
    </Icon>
  ),
  // sound waveform
  speech: (
    <Icon>
      <path d="M2 8h1.5M5 5v6M7.5 2.5v11M10 5v6M12.5 8H14" />
    </Icon>
  ),
  // sparkle
  ai: (
    <Icon>
      <>
        <path d="M8 2.5c.4 2.4 1.1 3.1 3.5 3.5-2.4.4-3.1 1.1-3.5 3.5-.4-2.4-1.1-3.1-3.5-3.5C6.9 5.6 7.6 4.9 8 2.5z" />
        <path d="M12.5 9.5c.2 1.1.5 1.4 1.6 1.6-1.1.2-1.4.5-1.6 1.6-.2-1.1-.5-1.4-1.6-1.6 1.1-.2 1.4-.5 1.6-1.6z" />
      </>
    </Icon>
  ),
  // stacked cards
  library: (
    <Icon>
      <>
        <rect x="2.5" y="5.5" width="11" height="8" rx="1" />
        <path d="M4 3.5h8M5.5 1.5h5" />
      </>
    </Icon>
  ),
  // gear
  general: (
    <Icon>
      <>
        <circle cx="8" cy="8" r="2.3" />
        <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.5 3.5l1.4 1.4M11.1 11.1l1.4 1.4M12.5 3.5l-1.4 1.4M4.9 11.1l-1.4 1.4" />
      </>
    </Icon>
  ),
  // info
  about: (
    <Icon>
      <>
        <circle cx="8" cy="8" r="6" />
        <path d="M8 7.3v4" />
        <circle cx="8" cy="4.9" r="0.6" fill="currentColor" stroke="none" />
      </>
    </Icon>
  ),
};
