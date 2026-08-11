import type { Direction, OverlayMode, ShortcutAction } from "./ipc";

export interface SourceSettings {
  enabled: boolean;
  deviceId: string | null;
  direction: Direction;
}

export interface OverlaySettings {
  /** 0.2 – 1.0 background opacity of the overlay card */
  opacity: number;
  /** base font size in px */
  fontSize: number;
  mode: OverlayMode;
  clickThrough: boolean;
}

export interface AppSettings {
  sttModel: string;
  translationModel: string;
  system: SourceSettings;
  mic: SourceSettings;
  overlay: OverlaySettings;
  shortcuts: Record<ShortcutAction, string>;
  translationStyle: "natural" | "literal";
  /** Use OpenAI server-side turn detection (streams partials while speaking). */
  useServerVad: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  sttModel: "gpt-live-transcribe",
  translationModel: "gpt-4o-mini",
  system: { enabled: true, deviceId: null, direction: "auto_uz" },
  mic: { enabled: false, deviceId: null, direction: "auto_en" },
  overlay: { opacity: 0.85, fontSize: 16, mode: "interview", clickThrough: false },
  shortcuts: {
    toggle_overlay: "Ctrl+Shift+O",
    start_stop: "Ctrl+Shift+S",
    swap_direction: "Ctrl+Shift+D",
    pause_resume: "Ctrl+Shift+P",
    clear_history: "Ctrl+Shift+X",
  },
  translationStyle: "natural",
  useServerVad: true,
};

export function mergeSettings(saved: unknown): AppSettings {
  const s = (saved ?? {}) as Partial<AppSettings>;
  return {
    ...DEFAULT_SETTINGS,
    ...s,
    system: { ...DEFAULT_SETTINGS.system, ...s.system },
    mic: { ...DEFAULT_SETTINGS.mic, ...s.mic },
    overlay: { ...DEFAULT_SETTINGS.overlay, ...s.overlay },
    shortcuts: { ...DEFAULT_SETTINGS.shortcuts, ...s.shortcuts },
  };
}

export const DIRECTION_LABELS: Record<Direction, string> = {
  en_uz: "EN → UZ",
  uz_en: "UZ → EN",
  auto_uz: "AUTO → UZ",
  auto_en: "AUTO → EN",
};

export const SWAPPED_DIRECTION: Record<Direction, Direction> = {
  en_uz: "uz_en",
  uz_en: "en_uz",
  auto_uz: "auto_en",
  auto_en: "auto_uz",
};
