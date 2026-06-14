/** Pure presentation logic for the Transform page: search filtering. */

import type { Transform } from '@velata/core';

/**
 * Filters transforms by a search query, matching case-insensitively against
 * both `name` and `instruction`. A blank query (after trimming) returns every
 * transform. Order is preserved; transforms carry a stable `id`, so edit and
 * delete address rows by id rather than by list position.
 */
export function filterTransforms(transforms: readonly Transform[], query: string): Transform[] {
  const needle = query.trim().toLowerCase();
  if (needle === '') return [...transforms];
  return transforms.filter(
    (transform) =>
      transform.name.toLowerCase().includes(needle) ||
      transform.instruction.toLowerCase().includes(needle),
  );
}
