import { describe, expect, it } from 'vitest';
import { downloadFailed, isDownloading } from './modelStatus.js';

const progress = (
  done: boolean,
  error: string | null,
): {
  modelId: string;
  downloadedBytes: number;
  totalBytes: number;
  done: boolean;
  error: string | null;
} => ({
  modelId: 'base.en',
  downloadedBytes: 0,
  totalBytes: 1,
  done,
  error,
});

describe('isDownloading', () => {
  it('is true for a registry flag or unfinished progress', () => {
    expect(isDownloading({ downloading: true }, undefined)).toBe(true);
    expect(isDownloading({ downloading: false }, progress(false, null))).toBe(true);
    expect(isDownloading(undefined, progress(false, null))).toBe(true);
  });

  it('is false when idle or finished', () => {
    expect(isDownloading({ downloading: false }, undefined)).toBe(false);
    expect(isDownloading(undefined, undefined)).toBe(false);
    expect(isDownloading({ downloading: false }, progress(true, null))).toBe(false);
  });
});

describe('downloadFailed', () => {
  it('is true only for a finished download with an error', () => {
    expect(downloadFailed(progress(true, 'network'))).toBe(true);
    expect(downloadFailed(progress(true, null))).toBe(false);
    expect(downloadFailed(progress(false, 'network'))).toBe(false);
    expect(downloadFailed(undefined)).toBe(false);
  });
});
