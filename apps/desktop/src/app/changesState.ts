import type { PipelineStatus, TranscriptionResult } from '@velata/core';

/** What the changes overlay is currently showing. */
export interface ChangesState {
  result: TranscriptionResult | null;
  visible: boolean;
}

export type ChangesEvent =
  | { type: 'toggle'; result: TranscriptionResult }
  | { type: 'pipeline'; status: PipelineStatus }
  | { type: 'close' };

export const initialChangesState: ChangesState = { result: null, visible: false };

/**
 * Pure transition for the overlay. The hotkey flips visibility (always against
 * the latest result); the Close button and any new pipeline activity hide it,
 * so a stale diff never lingers over fresh work.
 */
export function nextChangesState(prev: ChangesState, event: ChangesEvent): ChangesState {
  switch (event.type) {
    case 'toggle':
      return { result: event.result, visible: !prev.visible };
    case 'pipeline':
      return event.status === 'idle' ? prev : { ...prev, visible: false };
    case 'close':
      return { ...prev, visible: false };
  }
}
