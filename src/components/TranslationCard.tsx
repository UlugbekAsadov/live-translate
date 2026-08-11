import type { Segment } from "../stores/sessionStore";

function timeOf(ts: number): string {
  return new Date(ts).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function TranslationCard({
  segment,
  compact,
}: {
  segment: Segment;
  compact?: boolean;
}) {
  const badge = segment.source === "system" ? "SYS" : "MIC";
  const badgeColor =
    segment.source === "system"
      ? "bg-sky-500/20 text-sky-300"
      : "bg-purple-500/20 text-purple-300";

  return (
    <div className="rounded-lg bg-white/5 px-3 py-2">
      {!compact && (
        <div className="mb-1 flex items-center gap-2 text-[10px] text-slate-400">
          <span className={`rounded px-1 py-px font-semibold ${badgeColor}`}>{badge}</span>
          <span>{timeOf(segment.ts)}</span>
        </div>
      )}
      <p
        className={`text-[0.8em] leading-snug text-slate-400 ${
          segment.final ? "" : "italic opacity-70"
        }`}
      >
        {segment.text}
      </p>
      {segment.translation && (
        <p
          className={`mt-1 leading-snug text-slate-100 ${
            segment.translationFinal ? "" : "opacity-80"
          }`}
        >
          {segment.translation}
        </p>
      )}
    </div>
  );
}
