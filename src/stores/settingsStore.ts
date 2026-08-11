import { create } from "zustand";
import { load, type Store } from "@tauri-apps/plugin-store";
import { emit } from "@tauri-apps/api/event";
import { DEFAULT_SETTINGS, mergeSettings, type AppSettings } from "../types/settings";

let storeFile: Store | null = null;

interface SettingsStoreState {
  settings: AppSettings;
  loaded: boolean;
  /** Load persisted settings (call once per window at startup). */
  init: () => Promise<void>;
  /** Update, persist, and broadcast to the other window. */
  update: (patch: (s: AppSettings) => AppSettings) => Promise<void>;
  /** Apply settings that another window persisted (no re-save, no re-broadcast). */
  applyExternal: (s: AppSettings) => void;
}

export const useSettingsStore = create<SettingsStoreState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loaded: false,

  init: async () => {
    if (get().loaded) return;
    try {
      storeFile = await load("settings.json", { autoSave: false });
      const saved = await storeFile.get<AppSettings>("settings");
      set({ settings: mergeSettings(saved), loaded: true });
    } catch (e) {
      console.error("failed to load settings, using defaults", e);
      set({ loaded: true });
    }
  },

  update: async (patch) => {
    const next = patch(get().settings);
    set({ settings: next });
    try {
      if (storeFile) {
        await storeFile.set("settings", next);
        await storeFile.save();
      }
      await emit("settings:changed", next);
    } catch (e) {
      console.error("failed to persist settings", e);
    }
  },

  applyExternal: (s) => set({ settings: mergeSettings(s) }),
}));
