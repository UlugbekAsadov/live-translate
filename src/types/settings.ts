import type { ShortcutAction, OverlayMode } from "./ipc";

export interface LanguageOption {
  /** ISO 639-1 code, or "auto" (source only) */
  code: string;
  label: string;
}

/** Target languages offered in the dropdowns. Source adds "Auto Detect". */
export const LANGUAGES: LanguageOption[] = [
  { code: "uz", label: "Uzbek" },
  { code: "en", label: "English" },
  { code: "ru", label: "Russian" },
  { code: "tr", label: "Turkish" },
  { code: "kk", label: "Kazakh" },
  { code: "ky", label: "Kyrgyz" },
  { code: "tg", label: "Tajik" },
  { code: "az", label: "Azerbaijani" },
  { code: "ar", label: "Arabic" },
  { code: "fa", label: "Persian" },
  { code: "es", label: "Spanish" },
  { code: "fr", label: "French" },
  { code: "de", label: "German" },
  { code: "it", label: "Italian" },
  { code: "pt", label: "Portuguese" },
  { code: "hi", label: "Hindi" },
  { code: "zh", label: "Chinese" },
  { code: "ja", label: "Japanese" },
  { code: "ko", label: "Korean" },
  { code: "uk", label: "Ukrainian" },
  { code: "id", label: "Indonesian" },
  { code: "vi", label: "Vietnamese" },
];

export interface SourceSettings {
  enabled: boolean;
  deviceId: string | null;
  /** "auto" or ISO 639-1 code */
  sourceLang: string;
  targetLang: string;
  /** previous target — used by "swap" when sourceLang is "auto" */
  altTargetLang: string;
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
  system: {
    enabled: true,
    deviceId: null,
    sourceLang: "auto",
    targetLang: "uz",
    altTargetLang: "en",
  },
  mic: {
    enabled: false,
    deviceId: null,
    sourceLang: "auto",
    targetLang: "en",
    altTargetLang: "uz",
  },
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

/** Settings saved before the multilanguage dropdowns used a `direction` enum. */
const LEGACY_DIRECTIONS: Record<string, { sourceLang: string; targetLang: string }> = {
  en_uz: { sourceLang: "en", targetLang: "uz" },
  uz_en: { sourceLang: "uz", targetLang: "en" },
  auto_uz: { sourceLang: "auto", targetLang: "uz" },
  auto_en: { sourceLang: "auto", targetLang: "en" },
};

function mergeSource(
  defaults: SourceSettings,
  saved: (Partial<SourceSettings> & { direction?: string }) | undefined,
): SourceSettings {
  const merged = { ...defaults, ...saved };
  if (saved?.direction && !saved.sourceLang) {
    const legacy = LEGACY_DIRECTIONS[saved.direction];
    if (legacy) {
      merged.sourceLang = legacy.sourceLang;
      merged.targetLang = legacy.targetLang;
      merged.altTargetLang = legacy.targetLang === "uz" ? "en" : "uz";
    }
  }
  delete (merged as { direction?: string }).direction;
  return merged;
}

export function mergeSettings(saved: unknown): AppSettings {
  const s = (saved ?? {}) as Partial<AppSettings> & {
    system?: Partial<SourceSettings> & { direction?: string };
    mic?: Partial<SourceSettings> & { direction?: string };
  };
  return {
    ...DEFAULT_SETTINGS,
    ...s,
    system: mergeSource(DEFAULT_SETTINGS.system, s.system),
    mic: mergeSource(DEFAULT_SETTINGS.mic, s.mic),
    overlay: { ...DEFAULT_SETTINGS.overlay, ...s.overlay },
    shortcuts: { ...DEFAULT_SETTINGS.shortcuts, ...s.shortcuts },
  };
}

/**
 * Swap the translation direction of one source.
 * - fixed source: classic swap (from ⇄ to);
 * - auto source: stays auto, target toggles with the remembered alternate.
 */
export function swapPair(s: SourceSettings): SourceSettings {
  if (s.sourceLang === "auto") {
    return { ...s, targetLang: s.altTargetLang, altTargetLang: s.targetLang };
  }
  return { ...s, sourceLang: s.targetLang, targetLang: s.sourceLang };
}

export function pairLabel(s: SourceSettings): string {
  const from = s.sourceLang === "auto" ? "AUTO" : s.sourceLang.toUpperCase();
  return `${from} → ${s.targetLang.toUpperCase()}`;
}
