/**
 * Local release notes shown on the About tab. Wispr Flow fetches "What's new"
 * from a cloud Help Center; Velata ships the notes in the binary so the parity
 * feature stays true to the no-network promise. Newest entry first.
 */
export interface ReleaseNote {
  version: string;
  /** ISO date (YYYY-MM-DD) of the release. */
  date: string;
  /** Short, sentence-case bullets, each stating what the user can now do. */
  highlights: string[];
}

export const WHATS_NEW: ReleaseNote[] = [
  {
    version: '0.1.0',
    date: '2026-06-13',
    highlights: [
      'Find every feature through a two-section sidebar that splits Features from Settings.',
      'Review past dictations on Home, grouped by day.',
      'Track your usage on a local Insights page with streaks and per-app categories.',
      'Manage spoken terms in a searchable, editable Dictionary.',
      'Expand shorthands with Snippets you can search and edit.',
      'Set per-app writing Style with cleanup levels.',
      'Pick a polish from the Transforms shelf, tune the built-in Polish rules, or open the Prompt Engineer.',
      'Jot quick notes in an opt-in Scratchpad window.',
    ],
  },
  {
    version: '0.1.0',
    date: '2026-06-12',
    highlights: ['Velata replaces OpenFlow as the app name across the interface and data folder.'],
  },
  {
    version: '0.1.0',
    date: '2026-06-12',
    highlights: ['Velata windows now follow your macOS light or dark theme.'],
  },
];
