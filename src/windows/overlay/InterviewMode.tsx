import { useSegments, useSessionStore } from "../../stores/sessionStore";

/**
 * Compact mode for live interviews: the latest translation is prominent,
 * with up to two previous items dimmed above it. Partial transcripts render
 * live so the overlay never feels dead while someone is speaking.
 */
export function InterviewMode() {
  const segments = useSegments();
  const lastLatencyMs = useSessionStore((s) => s.lastLatencyMs);

  const recent = segments.slice(-3);
  const latest = recent[recent.length - 1];
  const previous = recent.slice(0, -1);

  return (
    <div className="flex min-h-0 flex-1 flex-col justify-end gap-1 overflow-hidden p-3">
      {previous.map((seg) => (
        <p key={seg.id} className="truncate text-[0.75em] leading-snug text-slate-500">
          {seg.translation || seg.text}
        </p>
      ))}
      {latest ? (
        <>
          <p className="text-[1.15em] font-medium leading-snug text-slate-50">
            {latest.translation || (
              <span className="italic text-slate-300">{latest.text}</span>
            )}
          </p>
          {lastLatencyMs != null && (
            <p className="text-[0.65em] text-slate-500">
              {(lastLatencyMs / 1000).toFixed(1)}s
            </p>
          )}
        </>
      ) : (
        <p className="text-[0.85em] text-slate-500">Waiting for speech…</p>
      )}
    </div>
  );
}
