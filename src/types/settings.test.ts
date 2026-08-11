import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, mergeSettings, swapPair } from "./settings";
import { formatShortcut } from "../components/ShortcutRecorder";

describe("mergeSettings", () => {
  it("returns defaults for empty input", () => {
    expect(mergeSettings(undefined)).toEqual(DEFAULT_SETTINGS);
    expect(mergeSettings(null)).toEqual(DEFAULT_SETTINGS);
  });

  it("keeps saved values and fills missing nested keys", () => {
    const merged = mergeSettings({
      sttModel: "custom-model",
      system: { sourceLang: "en", targetLang: "ru" },
    });
    expect(merged.sttModel).toBe("custom-model");
    expect(merged.system.sourceLang).toBe("en");
    expect(merged.system.targetLang).toBe("ru");
    expect(merged.system.enabled).toBe(DEFAULT_SETTINGS.system.enabled);
    expect(merged.shortcuts.start_stop).toBe(DEFAULT_SETTINGS.shortcuts.start_stop);
  });

  it("migrates legacy direction enums to language pairs", () => {
    const merged = mergeSettings({
      system: { direction: "en_uz" },
      mic: { direction: "auto_en" },
    });
    expect(merged.system.sourceLang).toBe("en");
    expect(merged.system.targetLang).toBe("uz");
    expect(merged.mic.sourceLang).toBe("auto");
    expect(merged.mic.targetLang).toBe("en");
    expect("direction" in merged.system).toBe(false);
  });
});

describe("swapPair", () => {
  it("swaps source and target for fixed languages", () => {
    const s = swapPair({ ...DEFAULT_SETTINGS.system, sourceLang: "en", targetLang: "uz" });
    expect(s.sourceLang).toBe("uz");
    expect(s.targetLang).toBe("en");
  });

  it("toggles target with the alternate when source is auto", () => {
    const base = {
      ...DEFAULT_SETTINGS.system,
      sourceLang: "auto",
      targetLang: "uz",
      altTargetLang: "en",
    };
    const once = swapPair(base);
    expect(once.sourceLang).toBe("auto");
    expect(once.targetLang).toBe("en");
    expect(once.altTargetLang).toBe("uz");
    expect(swapPair(once).targetLang).toBe("uz"); // involution
  });
});

describe("formatShortcut", () => {
  const base = { ctrlKey: false, shiftKey: false, altKey: false, metaKey: false };

  it("formats modifier + letter", () => {
    expect(formatShortcut({ ...base, ctrlKey: true, shiftKey: true, key: "o" })).toBe(
      "Ctrl+Shift+O",
    );
  });

  it("rejects pure modifier presses and unmodified keys", () => {
    expect(formatShortcut({ ...base, ctrlKey: true, key: "Control" })).toBeNull();
    expect(formatShortcut({ ...base, key: "a" })).toBeNull();
  });

  it("keeps named keys as-is", () => {
    expect(formatShortcut({ ...base, altKey: true, key: "F5" })).toBe("Alt+F5");
  });
});
