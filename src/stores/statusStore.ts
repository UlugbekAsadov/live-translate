import { create } from "zustand";
import type {
  AppErrorPayload,
  AudioLevelPayload,
  PipelineState,
  PipelineStatusPayload,
  Source,
} from "../types/ipc";

export interface SourceStatus {
  state: PipelineState;
  detail?: string;
  rms: number;
}

const IDLE: SourceStatus = { state: "idle", rms: 0 };

interface StatusState {
  system: SourceStatus;
  mic: SourceStatus;
  lastError: AppErrorPayload | null;
  setStatus: (p: PipelineStatusPayload) => void;
  setLevel: (p: AudioLevelPayload) => void;
  setError: (p: AppErrorPayload) => void;
  clearError: () => void;
}

export const useStatusStore = create<StatusState>((set) => ({
  system: IDLE,
  mic: IDLE,
  lastError: null,

  setStatus: (p) =>
    set((s) => ({
      [p.source]: { ...s[p.source], state: p.state, detail: p.detail },
    })),

  setLevel: (p) => set((s) => ({ [p.source]: { ...s[p.source], rms: p.rms } })),

  setError: (p) => set({ lastError: p }),
  clearError: () => set({ lastError: null }),
}));

const ACTIVE_STATES: PipelineState[] = [
  "starting",
  "listening",
  "speech",
  "paused",
  "reconnecting",
];

export function isSourceActive(status: SourceStatus): boolean {
  return ACTIVE_STATES.includes(status.state);
}

export function activeSources(s: {
  system: SourceStatus;
  mic: SourceStatus;
}): Source[] {
  const out: Source[] = [];
  if (isSourceActive(s.system)) out.push("system");
  if (isSourceActive(s.mic)) out.push("mic");
  return out;
}
