import { useCallback, useEffect, useState } from 'react';
import type {
  DownloadProgress,
  ModelInfo,
  PermissionsState,
  PipelineState,
  Settings,
  TranscriptionResult,
} from '@openflow/core';
import { events, ipc, subscribe } from './ipc.js';

export interface SettingsApi {
  settings: Settings;
  /** Saves the whole settings object; returns false (and sets `saveError`) on rejection. */
  save: (next: Settings) => Promise<boolean>;
  /** Shallow-merge convenience for single-field updates. */
  update: (patch: Partial<Settings>) => Promise<boolean>;
  saveError: string | null;
  dismissError: () => void;
}

export function useSettings(): SettingsApi | null {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    void ipc.getSettings().then(setSettings);
    return subscribe(events.onSettingsChanged(setSettings));
  }, []);

  const save = useCallback(async (next: Settings): Promise<boolean> => {
    setSettings(next); // optimistic; settings-changed event is the authority
    try {
      const saved = await ipc.saveSettings(next);
      setSettings(saved);
      setSaveError(null);
      return true;
    } catch (err) {
      setSaveError(String(err));
      setSettings(await ipc.getSettings());
      return false;
    }
  }, []);

  const dismissError = useCallback(() => {
    setSaveError(null);
  }, []);

  if (!settings) return null;
  return {
    settings,
    save,
    update: (patch) => save({ ...settings, ...patch }),
    saveError,
    dismissError,
  };
}

export function usePipeline(): { state: PipelineState; lastResult: TranscriptionResult | null } {
  const [state, setState] = useState<PipelineState>({ status: 'idle', job: null, message: null });
  const [lastResult, setLastResult] = useState<TranscriptionResult | null>(null);

  useEffect(() => {
    void ipc.getPipelineState().then(setState);
    void ipc.getLastResult().then(setLastResult);
    const cleanups = [
      subscribe(events.onPipelineState(setState)),
      subscribe(events.onResult(setLastResult)),
    ];
    return () => {
      cleanups.forEach((fn) => {
        fn();
      });
    };
  }, []);

  return { state, lastResult };
}

export interface ModelsApi {
  models: ModelInfo[];
  progress: Record<string, DownloadProgress>;
  download: (modelId: string) => void;
  cancel: (modelId: string) => void;
  remove: (modelId: string) => Promise<void>;
}

export function useModels(): ModelsApi {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<Record<string, DownloadProgress>>({});

  const refresh = useCallback(() => {
    void ipc.listModels().then(setModels);
  }, []);

  useEffect(() => {
    refresh();
    return subscribe(
      events.onModelDownload((p) => {
        setProgress((prev) => ({ ...prev, [p.modelId]: p }));
        if (p.done) refresh();
      }),
    );
  }, [refresh]);

  return {
    models,
    progress,
    download: (modelId) => {
      setProgress((prev) => ({
        ...prev,
        [modelId]: { modelId, downloadedBytes: 0, totalBytes: 0, done: false, error: null },
      }));
      void ipc.downloadModel(modelId).then(refresh);
    },
    cancel: (modelId) => {
      void ipc.cancelModelDownload(modelId);
    },
    remove: async (modelId) => {
      await ipc.deleteModel(modelId);
      refresh();
    },
  };
}

/** Polls permissions while mounted — users flip them in System Settings. */
export function usePermissions(intervalMs = 1500): PermissionsState | null {
  const [permissions, setPermissions] = useState<PermissionsState | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = (): void => {
      void ipc.checkPermissions().then((p) => {
        if (!cancelled) setPermissions(p);
      });
    };
    tick();
    const timer = setInterval(tick, intervalMs);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [intervalMs]);

  return permissions;
}
