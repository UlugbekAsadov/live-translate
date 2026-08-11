import { useEffect, useRef, useState } from "react";
import { ipc } from "../services/ipc";
import { startAll, stopAll } from "../services/controller";
import { useSettings } from "../hooks/useSettings";
import { useSegments, useSessionStore } from "../stores/sessionStore";
import { isSourceActive, useStatusStore } from "../stores/statusStore";
import { AudioLevelMeter } from "../components/AudioLevelMeter";
import { DeviceSelector } from "../components/DeviceSelector";
import { DirectionToggle } from "../components/DirectionToggle";
import { StatusPill } from "../components/StatusPill";
import { TranslationCard } from "../components/TranslationCard";
import type { Source } from "../types/ipc";

function SourcePanel({ source, title }: { source: Source; title: string }) {
  const { settings, update } = useSettings();
  const status = useStatusStore((s) => s[source]);
  const cfg = settings[source];
  const running = isSourceActive(status);

  return (
    <div className="rounded-xl border border-white/10 bg-white/[0.03] p-4">
      <div className="mb-3 flex items-center justify-between">
        <label className="flex items-center gap-2 text-sm font-medium text-slate-200">
          <input
            type="checkbox"
            className="h-4 w-4 accent-emerald-400"
            checked={cfg.enabled}
            disabled={running}
            onChange={(e) =>
              update((s) => ({ ...s, [source]: { ...cfg, enabled: e.target.checked } }))
            }
          />
          {title}
        </label>
        <StatusPill state={status.state} detail={status.detail} />
      </div>
      <div className="space-y-2">
        <DeviceSelector
          source={source}
          value={cfg.deviceId}
          disabled={running || !cfg.enabled}
          onChange={(deviceId) => update((s) => ({ ...s, [source]: { ...cfg, deviceId } }))}
        />
        <DirectionToggle
          value={cfg.direction}
          disabled={!cfg.enabled}
          onChange={async (direction) => {
            await update((s) => ({ ...s, [source]: { ...cfg, direction } }));
            if (running) await ipc.setDirection(source, direction);
          }}
        />
        <AudioLevelMeter rms={status.rms} />
      </div>
    </div>
  );
}

export function Dashboard() {
  const system = useStatusStore((s) => s.system);
  const mic = useStatusStore((s) => s.mic);
  const lastError = useStatusStore((s) => s.lastError);
  const clearError = useStatusStore((s) => s.clearError);
  const segments = useSegments();
  const lastLatencyMs = useSessionStore((s) => s.lastLatencyMs);
  const { settings } = useSettings();

  const [hasKey, setHasKey] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const feedRef = useRef<HTMLDivElement>(null);

  const running = isSourceActive(system) || isSourceActive(mic);

  useEffect(() => {
    ipc.hasApiKey().then(setHasKey).catch(() => setHasKey(false));
  }, []);

  useEffect(() => {
    feedRef.current?.scrollTo({ top: feedRef.current.scrollHeight });
  }, [segments.length, segments[segments.length - 1]?.translation]);

  async function onStartStop() {
    setBusy(true);
    try {
      if (running) await stopAll();
      else await startAll();
    } catch (e) {
      useStatusStore.getState().setError({
        code: "internal",
        message: String(e),
        recoverable: true,
      });
    } finally {
      setBusy(false);
    }
  }

  const nothingEnabled = !settings.system.enabled && !settings.mic.enabled;

  return (
    <div className="grid h-full grid-cols-[340px_1fr] gap-4 p-4">
      <div className="flex flex-col gap-4 overflow-y-auto">
        {hasKey === false && (
          <div className="rounded-xl border border-amber-400/30 bg-amber-400/10 p-3 text-sm text-amber-200">
            No OpenAI API key configured. Add one in <b>Settings</b> before starting.
          </div>
        )}

        <SourcePanel source="system" title="System Audio (Meet / Teams / Zoom)" />
        <SourcePanel source="mic" title="Microphone" />

        <button
          type="button"
          disabled={busy || (!running && (nothingEnabled || hasKey === false))}
          onClick={onStartStop}
          className={`rounded-xl px-4 py-3 text-base font-semibold transition-colors disabled:opacity-40 ${
            running
              ? "bg-red-500/90 text-white hover:bg-red-500"
              : "bg-emerald-500 text-emerald-950 hover:bg-emerald-400"
          }`}
        >
          {running ? "■ Stop Translation" : "▶ Start Translation"}
        </button>

        <div className="flex items-center justify-between text-xs text-slate-400">
          <span>
            Latency:{" "}
            {lastLatencyMs != null ? `${(lastLatencyMs / 1000).toFixed(1)}s` : "—"}
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              className="rounded-md border border-white/10 bg-white/5 px-2 py-1 hover:bg-white/10"
              onClick={() => ipc.toggleOverlay()}
            >
              Overlay
            </button>
            <button
              type="button"
              className="rounded-md border border-white/10 bg-white/5 px-2 py-1 hover:bg-white/10"
              onClick={() => ipc.clearHistory()}
            >
              Clear
            </button>
          </div>
        </div>

        {lastError && (
          <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-200">
            <div className="mb-1 flex items-center justify-between">
              <b>{lastError.code}</b>
              <button type="button" onClick={clearError} className="text-red-300">
                ✕
              </button>
            </div>
            {lastError.message}
          </div>
        )}
      </div>

      <div
        ref={feedRef}
        className="flex flex-col gap-2 overflow-y-auto rounded-xl border border-white/10 bg-white/[0.02] p-3"
      >
        {segments.length === 0 ? (
          <p className="m-auto text-sm text-slate-500">
            Transcripts and translations will appear here.
          </p>
        ) : (
          segments.map((seg) => <TranslationCard key={seg.id} segment={seg} />)
        )}
      </div>
    </div>
  );
}
