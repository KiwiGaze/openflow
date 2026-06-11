/** Typed wrappers around Tauri IPC. All names come from `@openflow/core`. */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  COMMANDS,
  EVENTS,
  type AppInfo,
  type DownloadProgress,
  type FrontmostApp,
  type HistoryEntry,
  type LlmProfile,
  type LlmTestResult,
  type ModelInfo,
  type PermissionsState,
  type PipelineState,
  type Settings,
  type SttProfile,
  type TranscriptionResult,
} from '@openflow/core';

export const ipc = {
  getSettings: (): Promise<Settings> => invoke(COMMANDS.getSettings),
  saveSettings: (settings: Settings): Promise<Settings> =>
    invoke(COMMANDS.saveSettings, { settings }),
  listModels: (): Promise<ModelInfo[]> => invoke(COMMANDS.listModels),
  downloadModel: (modelId: string): Promise<void> => invoke(COMMANDS.downloadModel, { modelId }),
  cancelModelDownload: (modelId: string): Promise<void> =>
    invoke(COMMANDS.cancelModelDownload, { modelId }),
  deleteModel: (modelId: string): Promise<void> => invoke(COMMANDS.deleteModel, { modelId }),
  getPipelineState: (): Promise<PipelineState> => invoke(COMMANDS.getPipelineState),
  startDictation: (): Promise<void> => invoke(COMMANDS.startDictation),
  stopDictation: (): Promise<void> => invoke(COMMANDS.stopDictation),
  cancelDictation: (): Promise<void> => invoke(COMMANDS.cancelDictation),
  startRefineSelection: (): Promise<void> => invoke(COMMANDS.startRefineSelection),
  startPolishSelection: (): Promise<void> => invoke(COMMANDS.startPolishSelection),
  getLastResult: (): Promise<TranscriptionResult | null> => invoke(COMMANDS.getLastResult),
  getLastDictationApp: (): Promise<FrontmostApp | null> => invoke(COMMANDS.getLastDictationApp),
  getHistory: (): Promise<HistoryEntry[]> => invoke(COMMANDS.getHistory),
  clearHistory: (): Promise<void> => invoke(COMMANDS.clearHistory),
  reprocessHistory: (text: string, modeId: string): Promise<string> =>
    invoke(COMMANDS.reprocessHistory, { text, modeId }),
  testLlm: (profile: LlmProfile): Promise<LlmTestResult> => invoke(COMMANDS.testLlm, { profile }),
  testMode: (prompt: string, sample: string, transforms: boolean): Promise<string> =>
    invoke(COMMANDS.testMode, { prompt, sample, transforms }),
  listLlmProfiles: (): Promise<LlmProfile[]> => invoke(COMMANDS.listLlmProfiles),
  saveLlmProfile: (profile: LlmProfile): Promise<LlmProfile[]> =>
    invoke(COMMANDS.saveLlmProfile, { profile }),
  deleteLlmProfile: (id: string): Promise<LlmProfile[]> =>
    invoke(COMMANDS.deleteLlmProfile, { id }),
  revealLlmProfiles: (): Promise<void> => invoke(COMMANDS.revealLlmProfiles),
  listSttProfiles: (): Promise<SttProfile[]> => invoke(COMMANDS.listSttProfiles),
  saveSttProfile: (profile: SttProfile): Promise<SttProfile[]> =>
    invoke(COMMANDS.saveSttProfile, { profile }),
  deleteSttProfile: (id: string): Promise<SttProfile[]> =>
    invoke(COMMANDS.deleteSttProfile, { id }),
  revealSttProfiles: (): Promise<void> => invoke(COMMANDS.revealSttProfiles),
  exportMode: (filename: string, contents: string): Promise<void> =>
    invoke(COMMANDS.exportMode, { filename, contents }),
  exportDictionary: (contents: string): Promise<void> =>
    invoke(COMMANDS.exportDictionary, { contents }),
  listOllamaModels: (baseUrl: string): Promise<string[]> =>
    invoke(COMMANDS.listOllamaModels, { baseUrl }),
  checkPermissions: (): Promise<PermissionsState> => invoke(COMMANDS.checkPermissions),
  requestMicrophonePermission: (): Promise<void> => invoke(COMMANDS.requestMicrophonePermission),
  promptAccessibilityPermission: (): Promise<boolean> =>
    invoke(COMMANDS.promptAccessibilityPermission),
  openAccessibilitySettings: (): Promise<void> => invoke(COMMANDS.openAccessibilitySettings),
  openMicrophoneSettings: (): Promise<void> => invoke(COMMANDS.openMicrophoneSettings),
  getAppInfo: (): Promise<AppInfo> => invoke(COMMANDS.getAppInfo),
};

export const events = {
  onPipelineState: (cb: (state: PipelineState) => void): Promise<UnlistenFn> =>
    listen<PipelineState>(EVENTS.pipelineState, (e) => {
      cb(e.payload);
    }),
  onAudioLevel: (cb: (level: number) => void): Promise<UnlistenFn> =>
    listen<number>(EVENTS.audioLevel, (e) => {
      cb(e.payload);
    }),
  onModelDownload: (cb: (progress: DownloadProgress) => void): Promise<UnlistenFn> =>
    listen<DownloadProgress>(EVENTS.modelDownload, (e) => {
      cb(e.payload);
    }),
  onSettingsChanged: (cb: (settings: Settings) => void): Promise<UnlistenFn> =>
    listen<Settings>(EVENTS.settingsChanged, (e) => {
      cb(e.payload);
    }),
  onResult: (cb: (result: TranscriptionResult) => void): Promise<UnlistenFn> =>
    listen<TranscriptionResult>(EVENTS.result, (e) => {
      cb(e.payload);
    }),
};

/** Subscribes in an effect-friendly way: returns a cleanup function. */
export function subscribe(promise: Promise<UnlistenFn>): () => void {
  return () => {
    void promise.then((unlisten) => {
      unlisten();
    });
  };
}
