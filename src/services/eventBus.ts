import { listen } from "@tauri-apps/api/event";
import { useSessionStore } from "../stores/sessionStore";
import { useStatusStore } from "../stores/statusStore";
import { useSettingsStore } from "../stores/settingsStore";
import type {
  AppErrorPayload,
  AudioLevelPayload,
  PipelineStatusPayload,
  TranscriptPayload,
  TranslationDeltaPayload,
  TranslationFinalPayload,
} from "../types/ipc";
import type { AppSettings } from "../types/settings";

let started = false;

/**
 * Subscribes to all backend events exactly once per window and fans them
 * into the zustand stores. Called from both window entrypoints.
 */
export async function initEventBus(): Promise<void> {
  if (started) return;
  started = true;

  const session = () => useSessionStore.getState();
  const status = () => useStatusStore.getState();

  await Promise.all([
    listen<TranscriptPayload>("transcript:partial", (e) =>
      session().applyPartial(e.payload),
    ),
    listen<TranscriptPayload>("transcript:final", (e) =>
      session().applyFinal(e.payload),
    ),
    listen<TranslationDeltaPayload>("translation:delta", (e) =>
      session().applyTranslationDelta(e.payload),
    ),
    listen<TranslationFinalPayload>("translation:final", (e) =>
      session().applyTranslationFinal(e.payload),
    ),
    listen<PipelineStatusPayload>("pipeline:status", (e) =>
      status().setStatus(e.payload),
    ),
    listen<AudioLevelPayload>("audio:level", (e) => status().setLevel(e.payload)),
    listen<AppErrorPayload>("app:error", (e) => status().setError(e.payload)),
    listen("history:cleared", () => session().clear()),
    listen<AppSettings>("settings:changed", (e) =>
      useSettingsStore.getState().applyExternal(e.payload),
    ),
  ]);
}
