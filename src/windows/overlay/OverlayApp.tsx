import { useEffect } from "react";
import { getCurrentWindow, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";
import { load } from "@tauri-apps/plugin-store";
import { useSettings } from "../../hooks/useSettings";
import { useStatusStore } from "../../stores/statusStore";
import { DIRECTION_LABELS } from "../../types/settings";
import { FullMode } from "./FullMode";
import { InterviewMode } from "./InterviewMode";
import { OverlayControls } from "./OverlayControls";

interface Geometry {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Restore saved position/size, then persist changes (debounced). */
function useGeometryPersistence() {
  useEffect(() => {
    const win = getCurrentWindow();
    let timer: ReturnType<typeof setTimeout> | undefined;
    let unMoved: (() => void) | undefined;
    let unResized: (() => void) | undefined;
    let disposed = false;

    (async () => {
      const store = await load("overlay-geometry.json", { autoSave: false });

      const saved = await store.get<Geometry>("geom");
      if (saved) {
        await win.setPosition(new PhysicalPosition(saved.x, saved.y));
        await win.setSize(new PhysicalSize(saved.w, saved.h));
      }

      const save = async () => {
        const pos = await win.outerPosition();
        const size = await win.outerSize();
        await store.set("geom", {
          x: pos.x,
          y: pos.y,
          w: size.width,
          h: size.height,
        } satisfies Geometry);
        await store.save();
      };
      const schedule = () => {
        clearTimeout(timer);
        timer = setTimeout(() => void save().catch(console.error), 500);
      };

      unMoved = await win.onMoved(schedule);
      unResized = await win.onResized(schedule);
      if (disposed) {
        unMoved?.();
        unResized?.();
      }
    })().catch(console.error);

    return () => {
      disposed = true;
      clearTimeout(timer);
      unMoved?.();
      unResized?.();
    };
  }, []);
}

export function OverlayApp() {
  const { settings, loaded } = useSettings();
  const system = useStatusStore((s) => s.system);
  const mic = useStatusStore((s) => s.mic);
  useGeometryPersistence();

  if (!loaded) return null;

  const { overlay } = settings;
  const labels: string[] = [];
  if (settings.system.enabled) labels.push(DIRECTION_LABELS[settings.system.direction]);
  if (settings.mic.enabled) labels.push(`🎙 ${DIRECTION_LABELS[settings.mic.direction]}`);
  const reconnecting =
    system.state === "reconnecting" || mic.state === "reconnecting";

  return (
    <div
      className="group flex h-screen flex-col overflow-hidden rounded-xl border border-white/10"
      style={{
        background: `rgba(10, 13, 18, ${overlay.opacity})`,
        fontSize: overlay.fontSize,
      }}
    >
      <header
        data-tauri-drag-region
        className="flex shrink-0 cursor-move items-center justify-between px-3 py-1.5"
      >
        <span
          data-tauri-drag-region
          className="pointer-events-none text-[0.65em] font-semibold tracking-widest text-slate-400"
        >
          {labels.join("  ·  ") || "TRANSLATOR"}
          {reconnecting && <span className="ml-2 text-orange-400">reconnecting…</span>}
        </span>
        <OverlayControls />
      </header>
      {overlay.mode === "interview" ? <InterviewMode /> : <FullMode />}
    </div>
  );
}
