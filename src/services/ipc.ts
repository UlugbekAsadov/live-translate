import { invoke } from "@tauri-apps/api/core";
import type {
  DeviceList,
  OverlayMode,
  Source,
  StartPipelineParams,
  TestApiKeyResult,
} from "../types/ipc";

/** Typed wrappers for every Tauri command — the single source of truth for invoke() calls. */
export const ipc = {
  setApiKey: (key: string) => invoke<void>("set_api_key", { key }),
  hasApiKey: () => invoke<boolean>("has_api_key"),
  deleteApiKey: () => invoke<void>("delete_api_key"),
  testApiKey: () => invoke<TestApiKeyResult>("test_api_key"),

  listAudioDevices: () => invoke<DeviceList>("list_audio_devices"),

  startPipeline: (params: StartPipelineParams) =>
    invoke<void>("start_pipeline", { params }),
  stopPipeline: (source: Source) => invoke<void>("stop_pipeline", { source }),
  pausePipeline: (source: Source, paused: boolean) =>
    invoke<void>("pause_pipeline", { source, paused }),
  setDirection: (source: Source, sourceLang: string, targetLang: string) =>
    invoke<void>("set_direction", { source, sourceLang, targetLang }),

  toggleOverlay: () => invoke<boolean>("toggle_overlay"),
  setOverlayMode: (mode: OverlayMode) => invoke<void>("set_overlay_mode", { mode }),
  setOverlayClickThrough: (enabled: boolean) =>
    invoke<void>("set_overlay_click_through", { enabled }),

  applyShortcuts: (shortcuts: Record<string, string>) =>
    invoke<Record<string, boolean>>("apply_shortcuts", { shortcuts }),

  clearHistory: () => invoke<void>("clear_history"),
};
