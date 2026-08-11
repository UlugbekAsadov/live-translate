import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, mergeSettings, SWAPPED_DIRECTION } from "./settings";
import { formatShortcut } from "../components/ShortcutRecorder";

describe("mergeSettings", () => {
  it("returns defaults for empty input", () => {
    expect(mergeSettings(undefined)).toEqual(DEFAULT_SETTINGS);
    expect(mergeSettings(null)).toEqual(DEFAULT_SETTINGS);
  });

  it("keeps saved values and fills missing nested keys", () => {
    const merged = mergeSettings({
      sttModel: "custom-model",
      system: { direction: "en_uz" },
    });
    expect(merged.sttModel).toBe("custom-model");
    expect(merged.system.direction).toBe("en_uz");
    expect(merged.system.enabled).toBe(DEFAULT_SETTINGS.system.enabled);
    expect(merged.shortcuts.start_stop).toBe(DEFAULT_SETTINGS.shortcuts.start_stop);
  });
});

describe("SWAPPED_DIRECTION", () => {
  it("is an involution (swapping twice returns the original)", () => {
    for (const [from, to] of Object.entries(SWAPPED_DIRECTION)) {
      expect(SWAPPED_DIRECTION[to]).toBe(from);
    }
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
