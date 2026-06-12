import type { DownloadProgress, ModelInfo } from '@velata/core';

/** One copy of the failure hint so the wording cannot drift between screens. */
export const DOWNLOAD_FAILED_HINT = 'Download failed — check your connection.';

/**
 * True while a model download is in flight: the registry flag covers
 * downloads the backend already knows about, live progress covers the one
 * started in this session. ModelsTab and Onboarding must agree on this.
 */
export function isDownloading(
  model: Pick<ModelInfo, 'downloading'> | undefined,
  progress: DownloadProgress | undefined,
): boolean {
  return (model?.downloading ?? false) || (progress !== undefined && !progress.done);
}

/** True when the last download attempt for this model ended in an error. */
export function downloadFailed(progress: DownloadProgress | undefined): boolean {
  return Boolean(progress?.done && progress.error);
}
