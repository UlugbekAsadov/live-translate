import { useState } from "react";

/** Turns a KeyboardEvent into an accelerator string like "Ctrl+Shift+O". */
export function formatShortcut(e: {
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  key: string;
}): string | null {
  const key = e.key;
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  if (parts.length === 0) return null; // require at least one modifier
  parts.push(key.length === 1 ? key.toUpperCase() : key);
  return parts.join("+");
}

export function ShortcutRecorder({
  value,
  onChange,
  ok,
}: {
  value: string;
  onChange: (accel: string) => void;
  /** false when the last registration attempt failed (conflict) */
  ok?: boolean;
}) {
  const [recording, setRecording] = useState(false);

  return (
    <button
      type="button"
      className={`w-40 rounded-md border px-2 py-1.5 text-left text-sm ${
        recording
          ? "border-emerald-400/60 bg-emerald-400/10 text-emerald-300"
          : ok === false
            ? "border-red-500/60 bg-red-500/10 text-red-300"
            : "border-white/10 bg-white/5 text-slate-200 hover:bg-white/10"
      }`}
      title={ok === false ? "Could not register (conflict with another app)" : undefined}
      onFocus={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      onKeyDown={(e) => {
        e.preventDefault();
        const accel = formatShortcut(e);
        if (accel) {
          onChange(accel);
          setRecording(false);
          (e.target as HTMLElement).blur();
        }
      }}
    >
      {recording ? "Press keys…" : value}
    </button>
  );
}
