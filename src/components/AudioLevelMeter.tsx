export function AudioLevelMeter({ rms }: { rms: number }) {
  // RMS of speech is typically 0.02–0.3; scale so normal speech fills the bar.
  const pct = Math.min(100, Math.round(rms * 400));
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
      <div
        className="h-full rounded-full bg-emerald-400 transition-[width] duration-100"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}
