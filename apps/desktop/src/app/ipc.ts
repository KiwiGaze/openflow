/** Typed wrappers around Tauri IPC. All names come from `@velata/core`. */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  COMMANDS,
  EVENTS,
  type AppInfo,
  type DictionarySuggestion,
  type DownloadProgress,
  type FrontmostApp,
  type HistoryEntry,
  type Insights,
  type LlmProfile,
  type LlmTestResult,
  type ModelInfo,
  type Note,
  type NoteSummary,
  type NoteVersion,
  type PermissionsState,
  type PipelineState,
  type Settings,
  type SttProfile,
  type TranscriptionResult,
} from '@velata/core';
import type { TabId } from './sidebarTabs.js';

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
  startPolishSelection: (): Promise<void> => invoke(COMMANDS.startPolishSelection),
  getLastResult: (): Promise<TranscriptionResult | null> => invoke(COMMANDS.getLastResult),
  getLastDictationApp: (): Promise<FrontmostApp | null> => invoke(COMMANDS.getLastDictationApp),
  getHistory: (): Promise<HistoryEntry[]> => invoke(COMMANDS.getHistory),
  clearHistory: (): Promise<void> => invoke(COMMANDS.clearHistory),
  deleteHistoryEntry: (id: string): Promise<void> => invoke(COMMANDS.deleteHistoryEntry, { id }),
  reprocessHistory: (text: string, modeId: string): Promise<string> =>
    invoke(COMMANDS.reprocessHistory, { text, modeId }),
  getInsights: (): Promise<Insights> => invoke(COMMANDS.getInsights),
  listDictionarySuggestions: (): Promise<DictionarySuggestion[]> =>
    invoke(COMMANDS.listDictionarySuggestions),
  dismissDictionarySuggestion: (term: string): Promise<void> =>
    invoke(COMMANDS.dismissDictionarySuggestion, { term }),
  copyText: (text: string): Promise<void> => invoke(COMMANDS.copyText, { text }),
  setChangesInteractive: (interactive: boolean): Promise<void> =>
    invoke(COMMANDS.setChangesInteractive, { interactive }),
  testLlm: (profile: LlmProfile): Promise<LlmTestResult> => invoke(COMMANDS.testLlm, { profile }),
  testMode: (
    prompt: string,
    sample: string,
    transforms: boolean,
    aiProfileId: string | null,
  ): Promise<string> => invoke(COMMANDS.testMode, { prompt, sample, transforms, aiProfileId }),
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
  listInputDevices: (): Promise<string[]> => invoke(COMMANDS.listInputDevices),
  checkPermissions: (): Promise<PermissionsState> => invoke(COMMANDS.checkPermissions),
  requestMicrophonePermission: (): Promise<void> => invoke(COMMANDS.requestMicrophonePermission),
  promptAccessibilityPermission: (): Promise<boolean> =>
    invoke(COMMANDS.promptAccessibilityPermission),
  openAccessibilitySettings: (): Promise<void> => invoke(COMMANDS.openAccessibilitySettings),
  openMicrophoneSettings: (): Promise<void> => invoke(COMMANDS.openMicrophoneSettings),
  getAppInfo: (): Promise<AppInfo> => invoke(COMMANDS.getAppInfo),
  listNotes: (search: string | null): Promise<NoteSummary[]> =>
    invoke(COMMANDS.listNotes, { search }),
  getNote: (id: string): Promise<Note | null> => invoke(COMMANDS.getNote, { id }),
  createNote: (): Promise<Note> => invoke(COMMANDS.createNote),
  updateNote: (id: string, title: string, content: string): Promise<void> =>
    invoke(COMMANDS.updateNote, { id, title, content }),
  setNotePinned: (id: string, pinned: boolean): Promise<void> =>
    invoke(COMMANDS.setNotePinned, { id, pinned }),
  deleteNote: (id: string): Promise<void> => invoke(COMMANDS.deleteNote, { id }),
  listNoteVersions: (noteId: string): Promise<NoteVersion[]> =>
    invoke(COMMANDS.listNoteVersions, { noteId }),
  restoreNoteVersion: (versionId: string): Promise<Note> =>
    invoke(COMMANDS.restoreNoteVersion, { versionId }),
  transformNoteText: (noteId: string, transformId: string | null): Promise<Note> =>
    invoke(COMMANDS.transformNoteText, { noteId, transformId }),
  openScratchpadWindow: (noteId: string | null): Promise<void> =>
    invoke(COMMANDS.openScratchpadWindow, { noteId }),
  openMainWindow: (): Promise<void> => invoke(COMMANDS.openMainWindow),
  openSettingsWindow: (tab?: TabId): Promise<void> =>
    invoke(COMMANDS.openSettingsWindow, { tab: tab ?? null }),
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
  onChangesToggle: (cb: (result: TranscriptionResult) => void): Promise<UnlistenFn> =>
    listen<TranscriptionResult>(EVENTS.changesToggle, (e) => {
      cb(e.payload);
    }),
  /** History committed to the DB; refresh from durable rows (no payload). */
  onHistoryChanged: (cb: () => void): Promise<UnlistenFn> =>
    listen(EVENTS.historyChanged, () => {
      cb();
    }),
  /** A note changed; refresh the list from durable rows (no payload). */
  onNotesChanged: (cb: () => void): Promise<UnlistenFn> =>
    listen(EVENTS.notesChanged, () => {
      cb();
    }),
  /** An open Scratchpad is asked to switch to a note; payload is the note id. */
  onScratchpadOpenNote: (cb: (noteId: string) => void): Promise<UnlistenFn> =>
    listen<string>(EVENTS.scratchpadOpenNote, (e) => {
      cb(e.payload);
    }),
  /** The open Settings window is asked to switch to a tab; payload is the tab id. */
  onSettingsNavigate: (cb: (tab: string) => void): Promise<UnlistenFn> =>
    listen<string>(EVENTS.settingsNavigate, (e) => {
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
