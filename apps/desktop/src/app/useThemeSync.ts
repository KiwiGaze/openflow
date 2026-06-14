import { useEffect } from 'react';
import type { Appearance } from '@velata/core';

/**
 * Apply the theme override before content paints; `system` defers to the OS via
 * the CSS media query, so the dataset attribute is a no-op there. Shared by the
 * App and Settings windows so both react to an appearance change immediately.
 */
export function useThemeSync(appearance: Appearance | undefined): void {
  useEffect(() => {
    document.documentElement.dataset.theme = appearance ?? 'system';
  }, [appearance]);
}
