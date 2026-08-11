import { useEffect, useRef } from "react";
import { useSegments } from "../../stores/sessionStore";
import { TranslationCard } from "../../components/TranslationCard";

export function FullMode() {
  const segments = useSegments();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight });
  }, [segments.length, segments[segments.length - 1]?.translation]);

  return (
    <div ref={ref} className="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto p-2">
      {segments.length === 0 ? (
        <p className="m-auto text-[0.8em] text-slate-500">Waiting for speech…</p>
      ) : (
        segments.map((seg) => <TranslationCard key={seg.id} segment={seg} />)
      )}
    </div>
  );
}
