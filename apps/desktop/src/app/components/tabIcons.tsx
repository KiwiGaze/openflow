import type { JSX } from 'react';

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

export const TAB_ICONS: Record<string, JSX.Element> = {
  // microphone
  dictation: (
    <Icon>
      <>
        <rect x="6" y="1.5" width="4" height="7.5" rx="2" />
        <path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.5M5.5 14.5h5" />
      </>
    </Icon>
  ),
  // sliders
  modes: (
    <Icon>
      <>
        <path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11" />
        <circle cx="5.5" cy="4.5" r="1.4" fill="currentColor" stroke="none" />
        <circle cx="10.5" cy="8" r="1.4" fill="currentColor" stroke="none" />
        <circle cx="6.5" cy="11.5" r="1.4" fill="currentColor" stroke="none" />
      </>
    </Icon>
  ),
  // chip
  models: (
    <Icon>
      <>
        <rect x="4.5" y="4.5" width="7" height="7" rx="1" />
        <path d="M6.5 2.5v2M9.5 2.5v2M6.5 11.5v2M9.5 11.5v2M2.5 6.5h2M2.5 9.5h2M11.5 6.5h2M11.5 9.5h2" />
      </>
    </Icon>
  ),
  // arrow leaving a box
  output: (
    <Icon>
      <>
        <path d="M8.5 3.5H4.5a1 1 0 0 0-1 1v7a1 1 0 0 0 1 1h7a1 1 0 0 0 1-1V7.5" />
        <path d="M8 8l5-5M9.5 3H13v3.5" />
      </>
    </Icon>
  ),
  // book
  dictionary: (
    <Icon>
      <>
        <path d="M3.5 3a1 1 0 0 1 1-1H12a.5.5 0 0 1 .5.5v10a.5.5 0 0 1-.5.5H4.5a1 1 0 0 1-1-1z" />
        <path d="M3.5 11.5a1 1 0 0 1 1-1h8" />
      </>
    </Icon>
  ),
  // lightning (expansion)
  snippets: (
    <Icon>
      <path d="M8.5 1.5 3.5 9h3.5l-1 5.5 5.5-7.5H8z" />
    </Icon>
  ),
  // bar chart
  insights: (
    <Icon>
      <path d="M3 13.5V9M7 13.5V5.5M11 13.5V7.5M14 13.5H2" />
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
