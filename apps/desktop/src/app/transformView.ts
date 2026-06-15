/** Pure presentation logic for the Transform page: search filtering. */

import type { Prompt } from '@velata/core';

/**
 * Filters prompts by a search query, matching case-insensitively against both
 * `name` and `instruction`. A blank query (after trimming) returns every prompt.
 * Order is preserved; prompts carry a stable `id`, so edit and delete address
 * rows by id rather than by list position.
 */
export function filterPrompts(prompts: readonly Prompt[], query: string): Prompt[] {
  const needle = query.trim().toLowerCase();
  if (needle === '') return [...prompts];
  return prompts.filter(
    (prompt) =>
      prompt.name.toLowerCase().includes(needle) ||
      prompt.instruction.toLowerCase().includes(needle),
  );
}
