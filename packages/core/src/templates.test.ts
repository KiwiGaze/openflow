import { describe, expect, it } from 'vitest';
import { MODE_TEMPLATES } from './templates.js';

describe('MODE_TEMPLATES', () => {
  it('ships nine templates with unique ids', () => {
    expect(MODE_TEMPLATES).toHaveLength(9);
    expect(new Set(MODE_TEMPLATES.map((t) => t.id)).size).toBe(9);
  });

  it('only the translation template transforms (drops the no-translate default)', () => {
    const transforming = MODE_TEMPLATES.filter((t) => t.transforms).map((t) => t.id);
    expect(transforming).toEqual(['translation']);
  });

  it('every template carries a non-empty prompt', () => {
    for (const template of MODE_TEMPLATES) {
      expect(template.prompt.length).toBeGreaterThan(0);
    }
  });
});
