import { ipc } from "./ipc";
import { useSettingsStore } from "../stores/settingsStore";
import { activeSources, useStatusStore } from "../stores/statusStore";
import { SWAPPED_DIRECTION } from "../types/settings";
import type { ShortcutAction, Source } from "../types/ipc";

/**
 * High-level actions shared by the Dashboard UI and global-shortcut handling.
 * The frontend owns settings; the backend owns running pipelines.
 */

export function runningSources(): Source[] {
  return activeSources(useStatusStore.getState());
}

export async function startAll(): Promise<void> {
  const { settings } = useSettingsStore.getState();
  useStatusStore.getState().clearError();
  const targets: Source[] = [];
  if (settings.system.enabled) targets.push("system");
  if (settings.mic.enabled) targets.push("mic");
  for (const source of targets) {
    const src = settings[source];
    await ipc.startPipeline({
      source,
      deviceId: src.deviceId,
      direction: src.direction,
      sttModel: settings.sttModel,
      translationModel: settings.translationModel,
      useServerVad: settings.useServerVad,
      translationStyle: settings.translationStyle,
    });
  }
}

export async function stopAll(): Promise<void> {
  for (const source of runningSources()) {
    await ipc.stopPipeline(source);
  }
}

export async function toggleStartStop(): Promise<void> {
  if (runningSources().length > 0) await stopAll();
  else await startAll();
}

export async function pauseResume(): Promise<void> {
  const status = useStatusStore.getState();
  const running = runningSources();
  if (running.length === 0) return;
  const anyPaused = running.some((s) => status[s].state === "paused");
  for (const source of running) {
    await ipc.pausePipeline(source, !anyPaused);
  }
}

export async function swapDirections(): Promise<void> {
  await useSettingsStore.getState().update((s) => ({
    ...s,
    system: { ...s.system, direction: SWAPPED_DIRECTION[s.system.direction] },
    mic: { ...s.mic, direction: SWAPPED_DIRECTION[s.mic.direction] },
  }));
  const { settings } = useSettingsStore.getState();
  for (const source of runningSources()) {
    await ipc.setDirection(source, settings[source].direction);
  }
}

export async function handleShortcut(action: ShortcutAction): Promise<void> {
  switch (action) {
    case "toggle_overlay":
      await ipc.toggleOverlay();
      break;
    case "start_stop":
      await toggleStartStop();
      break;
    case "swap_direction":
      await swapDirections();
      break;
    case "pause_resume":
      await pauseResume();
      break;
    case "clear_history":
      await ipc.clearHistory();
      break;
  }
}
