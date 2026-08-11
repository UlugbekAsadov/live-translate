import type { PipelineState } from "../types/ipc";

const COLORS: Record<PipelineState, string> = {
  idle: "bg-slate-500",
  starting: "bg-amber-400 animate-pulse",
  listening: "bg-emerald-400",
  speech: "bg-emerald-300 animate-pulse",
  paused: "bg-amber-400",
  reconnecting: "bg-orange-400 animate-pulse",
  error: "bg-red-500",
};

const LABELS: Record<PipelineState, string> = {
  idle: "Idle",
  starting: "Starting…",
  listening: "Listening",
  speech: "Speech",
  paused: "Paused",
  reconnecting: "Reconnecting…",
  error: "Error",
};

export function StatusPill({ state, detail }: { state: PipelineState; detail?: string }) {
  return (
    <span
      className="inline-flex items-center gap-1.5 rounded-full bg-white/5 px-2.5 py-0.5 text-xs text-slate-300"
      title={detail}
    >
      <span className={`h-2 w-2 rounded-full ${COLORS[state]}`} />
      {LABELS[state]}
    </span>
  );
}
