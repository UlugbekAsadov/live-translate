import { ipc } from "../../services/ipc";
import { useSettings } from "../../hooks/useSettings";

/** Hover-revealed controls in the overlay header. */
export function OverlayControls() {
  const { settings, update } = useSettings();
  const { overlay } = settings;

  return (
    <div className="flex items-center gap-2 opacity-0 transition-opacity group-hover:opacity-100">
      <input
        type="range"
        min={20}
        max={100}
        title="Opacity"
        className="w-14 accent-emerald-400"
        value={Math.round(overlay.opacity * 100)}
        onChange={(e) =>
          update((s) => ({
            ...s,
            overlay: { ...s.overlay, opacity: Number(e.target.value) / 100 },
          }))
        }
      />
      <button
        type="button"
        title="Smaller text"
        className="rounded px-1 text-xs text-slate-400 hover:text-slate-200"
        onClick={() =>
          update((s) => ({
            ...s,
            overlay: { ...s.overlay, fontSize: Math.max(12, s.overlay.fontSize - 1) },
          }))
        }
      >
        A−
      </button>
      <button
        type="button"
        title="Larger text"
        className="rounded px-1 text-xs text-slate-400 hover:text-slate-200"
        onClick={() =>
          update((s) => ({
            ...s,
            overlay: { ...s.overlay, fontSize: Math.min(28, s.overlay.fontSize + 1) },
          }))
        }
      >
        A+
      </button>
      <button
        type="button"
        title={overlay.mode === "interview" ? "Full history" : "Interview mode"}
        className="rounded px-1 text-xs text-slate-400 hover:text-slate-200"
        onClick={async () => {
          const mode = overlay.mode === "interview" ? "full" : "interview";
          await update((s) => ({ ...s, overlay: { ...s.overlay, mode } }));
          await ipc.setOverlayMode(mode);
        }}
      >
        {overlay.mode === "interview" ? "▤" : "▭"}
      </button>
      <button
        type="button"
        title="Hide overlay"
        className="rounded px-1 text-xs text-slate-400 hover:text-red-300"
        onClick={() => ipc.toggleOverlay()}
      >
        ✕
      </button>
    </div>
  );
}
