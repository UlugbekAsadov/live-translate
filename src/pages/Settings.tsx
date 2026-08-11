import { useState } from "react";
import { ApiKeyForm } from "../components/ApiKeyForm";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { ipc } from "../services/ipc";
import { useSettings } from "../hooks/useSettings";
import type { ShortcutAction } from "../types/ipc";

const SHORTCUT_LABELS: Record<ShortcutAction, string> = {
  toggle_overlay: "Show / hide overlay",
  start_stop: "Start / stop translation",
  swap_direction: "Swap translation direction",
  pause_resume: "Pause / resume",
  clear_history: "Clear history",
};

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded-xl border border-white/10 bg-white/[0.03] p-4">
      <h2 className="mb-3 text-sm font-semibold text-slate-200">{title}</h2>
      {children}
    </section>
  );
}

export function Settings() {
  const { settings, update } = useSettings();
  const [shortcutResults, setShortcutResults] = useState<Record<string, boolean>>({});

  async function applyShortcuts(shortcuts: Record<ShortcutAction, string>) {
    try {
      const results = await ipc.applyShortcuts(shortcuts);
      setShortcutResults(results);
    } catch (e) {
      console.error("failed to apply shortcuts", e);
    }
  }

  return (
    <div className="mx-auto flex max-w-2xl flex-col gap-4 p-4">
      <Section title="OpenAI API Key">
        <ApiKeyForm />
      </Section>

      <Section title="Models">
        <div className="grid grid-cols-2 gap-3">
          <label className="text-xs text-slate-400">
            Speech-to-text model
            <input
              className="mt-1 w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-200 outline-none focus:border-emerald-400/50"
              value={settings.sttModel}
              list="stt-models"
              onChange={(e) => update((s) => ({ ...s, sttModel: e.target.value }))}
            />
            <datalist id="stt-models">
              <option value="gpt-live-transcribe" />
              <option value="gpt-transcribe" />
            </datalist>
          </label>
          <label className="text-xs text-slate-400">
            Translation model
            <input
              className="mt-1 w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-200 outline-none focus:border-emerald-400/50"
              value={settings.translationModel}
              list="translation-models"
              onChange={(e) => update((s) => ({ ...s, translationModel: e.target.value }))}
            />
            <datalist id="translation-models">
              <option value="gpt-4o-mini" />
              <option value="gpt-4o" />
            </datalist>
          </label>
        </div>
        <p className="mt-2 text-xs text-slate-500">
          Model changes apply the next time translation is started.
        </p>
      </Section>

      <Section title="Translation">
        <div className="flex items-center gap-6">
          <label className="text-xs text-slate-400">
            Style
            <select
              className="mt-1 block rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-200"
              value={settings.translationStyle}
              onChange={(e) =>
                update((s) => ({
                  ...s,
                  translationStyle: e.target.value as "natural" | "literal",
                }))
              }
            >
              <option value="natural">Natural (recommended)</option>
              <option value="literal">More literal</option>
            </select>
          </label>
          <label className="flex items-center gap-2 text-xs text-slate-400">
            <input
              type="checkbox"
              className="h-4 w-4 accent-emerald-400"
              checked={settings.useServerVad}
              onChange={(e) => update((s) => ({ ...s, useServerVad: e.target.checked }))}
            />
            Server-side turn detection (streams partial words; disable only for
            troubleshooting)
          </label>
        </div>
      </Section>

      <Section title="Overlay">
        <div className="grid grid-cols-2 gap-4">
          <label className="text-xs text-slate-400">
            Opacity: {Math.round(settings.overlay.opacity * 100)}%
            <input
              type="range"
              min={20}
              max={100}
              className="mt-1 w-full accent-emerald-400"
              value={Math.round(settings.overlay.opacity * 100)}
              onChange={(e) =>
                update((s) => ({
                  ...s,
                  overlay: { ...s.overlay, opacity: Number(e.target.value) / 100 },
                }))
              }
            />
          </label>
          <label className="text-xs text-slate-400">
            Font size: {settings.overlay.fontSize}px
            <input
              type="range"
              min={12}
              max={28}
              className="mt-1 w-full accent-emerald-400"
              value={settings.overlay.fontSize}
              onChange={(e) =>
                update((s) => ({
                  ...s,
                  overlay: { ...s.overlay, fontSize: Number(e.target.value) },
                }))
              }
            />
          </label>
          <label className="text-xs text-slate-400">
            Mode
            <select
              className="mt-1 block w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-200"
              value={settings.overlay.mode}
              onChange={async (e) => {
                const mode = e.target.value as "full" | "interview";
                await update((s) => ({ ...s, overlay: { ...s.overlay, mode } }));
                await ipc.setOverlayMode(mode);
              }}
            >
              <option value="interview">Interview (compact)</option>
              <option value="full">Full history</option>
            </select>
          </label>
          <label className="mt-4 flex items-center gap-2 text-xs text-slate-400">
            <input
              type="checkbox"
              className="h-4 w-4 accent-emerald-400"
              checked={settings.overlay.clickThrough}
              onChange={async (e) => {
                const enabled = e.target.checked;
                await update((s) => ({
                  ...s,
                  overlay: { ...s.overlay, clickThrough: enabled },
                }));
                await ipc.setOverlayClickThrough(enabled);
              }}
            />
            Click-through (overlay ignores the mouse)
          </label>
        </div>
      </Section>

      <Section title="Keyboard Shortcuts">
        <div className="space-y-2">
          {(Object.keys(SHORTCUT_LABELS) as ShortcutAction[]).map((action) => (
            <div key={action} className="flex items-center justify-between">
              <span className="text-sm text-slate-300">{SHORTCUT_LABELS[action]}</span>
              <ShortcutRecorder
                value={settings.shortcuts[action]}
                ok={shortcutResults[action]}
                onChange={async (accel) => {
                  const next = { ...settings.shortcuts, [action]: accel };
                  await update((s) => ({ ...s, shortcuts: next }));
                  await applyShortcuts(next);
                }}
              />
            </div>
          ))}
        </div>
        <p className="mt-2 text-xs text-slate-500">
          Shortcuts are global — they work while Google Meet has focus. A red field means
          the combination is taken by another application.
        </p>
      </Section>
    </div>
  );
}
