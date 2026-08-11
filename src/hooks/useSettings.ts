import { useSettingsStore } from "../stores/settingsStore";
import type { AppSettings } from "../types/settings";

/** Convenience accessor for settings + updater. */
export function useSettings(): {
  settings: AppSettings;
  loaded: boolean;
  update: (patch: (s: AppSettings) => AppSettings) => Promise<void>;
} {
  const settings = useSettingsStore((s) => s.settings);
  const loaded = useSettingsStore((s) => s.loaded);
  const update = useSettingsStore((s) => s.update);
  return { settings, loaded, update };
}
