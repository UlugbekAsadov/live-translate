import type { Direction } from "../types/ipc";
import { DIRECTION_LABELS, SWAPPED_DIRECTION } from "../types/settings";

const OPTIONS: Direction[] = ["auto_uz", "auto_en", "en_uz", "uz_en"];

const FULL_LABELS: Record<Direction, string> = {
  en_uz: "English → Uzbek",
  uz_en: "Uzbek → English",
  auto_uz: "Auto Detect → Uzbek",
  auto_en: "Auto Detect → English",
};

export function DirectionToggle({
  value,
  onChange,
  disabled,
}: {
  value: Direction;
  onChange: (d: Direction) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center gap-2">
      <select
        className="flex-1 rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-200 outline-none focus:border-emerald-400/50 disabled:opacity-50"
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value as Direction)}
      >
        {OPTIONS.map((d) => (
          <option key={d} value={d}>
            {FULL_LABELS[d]}
          </option>
        ))}
      </select>
      <button
        type="button"
        title={`Swap (${DIRECTION_LABELS[SWAPPED_DIRECTION[value]]})`}
        disabled={disabled}
        onClick={() => onChange(SWAPPED_DIRECTION[value])}
        className="rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-300 hover:bg-white/10 disabled:opacity-50"
      >
        ⇄
      </button>
    </div>
  );
}
