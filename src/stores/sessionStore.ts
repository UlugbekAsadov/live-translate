import { useMemo } from "react";
import { create } from "zustand";
import type {
  Source,
  TranscriptPayload,
  TranslationDeltaPayload,
  TranslationFinalPayload,
} from "../types/ipc";

export interface Segment {
  id: string;
  source: Source;
  /** source-language transcript (partial until `final`) */
  text: string;
  final: boolean;
  translation: string;
  translationFinal: boolean;
  targetLang?: string;
  /** epoch ms when the segment was first seen */
  ts: number;
  translatedAt?: number;
}

const MAX_SEGMENTS = 200;

interface SessionState {
  order: string[];
  segments: Record<string, Segment>;
  lastLatencyMs: number | null;
  applyPartial: (p: TranscriptPayload) => void;
  applyFinal: (p: TranscriptPayload) => void;
  applyTranslationDelta: (p: TranslationDeltaPayload) => void;
  applyTranslationFinal: (p: TranslationFinalPayload) => void;
  clear: () => void;
}

function upsert(
  state: Pick<SessionState, "order" | "segments">,
  p: TranscriptPayload,
  final: boolean,
): Pick<SessionState, "order" | "segments"> {
  const existing = state.segments[p.segmentId];
  const seg: Segment = existing
    ? { ...existing, text: p.text, final }
    : {
        id: p.segmentId,
        source: p.source,
        text: p.text,
        final,
        translation: "",
        translationFinal: false,
        ts: p.ts,
      };
  let order = existing ? state.order : [...state.order, p.segmentId];
  const segments = { ...state.segments, [p.segmentId]: seg };
  if (order.length > MAX_SEGMENTS) {
    const dropped = order.slice(0, order.length - MAX_SEGMENTS);
    order = order.slice(order.length - MAX_SEGMENTS);
    for (const id of dropped) delete segments[id];
  }
  return { order, segments };
}

export const useSessionStore = create<SessionState>((set) => ({
  order: [],
  segments: {},
  lastLatencyMs: null,

  applyPartial: (p) => set((s) => upsert(s, p, false)),

  applyFinal: (p) => set((s) => upsert(s, p, true)),

  applyTranslationDelta: (p) =>
    set((s) => {
      const seg = s.segments[p.segmentId];
      if (!seg) return s;
      return {
        segments: {
          ...s.segments,
          [p.segmentId]: { ...seg, translation: seg.translation + p.delta },
        },
      };
    }),

  applyTranslationFinal: (p) =>
    set((s) => {
      const seg = s.segments[p.segmentId];
      if (!seg) return s;
      const translatedAt = Date.now();
      return {
        segments: {
          ...s.segments,
          [p.segmentId]: {
            ...seg,
            translation: p.text,
            translationFinal: true,
            targetLang: p.targetLang,
            translatedAt,
          },
        },
        lastLatencyMs: translatedAt - seg.ts,
      };
    }),

  clear: () => set({ order: [], segments: {}, lastLatencyMs: null }),
}));

/** Ordered segments, oldest first. */
export function selectSegments(s: SessionState): Segment[] {
  return s.order.map((id) => s.segments[id]).filter(Boolean);
}

/**
 * React hook for the ordered segment list. Selects the two stable references
 * and memoizes the derived array — a selector returning a fresh array every
 * call would make useSyncExternalStore re-render forever.
 */
export function useSegments(): Segment[] {
  const order = useSessionStore((s) => s.order);
  const segments = useSessionStore((s) => s.segments);
  return useMemo(() => order.map((id) => segments[id]).filter(Boolean), [order, segments]);
}
