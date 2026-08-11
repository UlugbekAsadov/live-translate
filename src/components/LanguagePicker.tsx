import { LANGUAGES, swapPair, type SourceSettings } from "../types/settings";

const selectClass =
  "min-w-0 flex-1 rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-200 outline-none focus:border-emerald-400/50 disabled:opacity-50";

export function LanguagePicker({
  value,
  onChange,
  disabled,
}: {
  value: SourceSettings;
  onChange: (next: SourceSettings) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center gap-1.5">
      <select
        className={selectClass}
        value={value.sourceLang}
        disabled={disabled}
        title="Source language"
        onChange={(e) => onChange({ ...value, sourceLang: e.target.value })}
      >
        <option value="auto">Auto Detect</option>
        {LANGUAGES.map((l) => (
          <option key={l.code} value={l.code}>
            {l.label}
          </option>
        ))}
      </select>
      <button
        type="button"
        title="Swap direction"
        disabled={disabled}
        onClick={() => onChange(swapPair(value))}
        className="shrink-0 rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-300 hover:bg-white/10 disabled:opacity-50"
      >
        ⇄
      </button>
      <select
        className={selectClass}
        value={value.targetLang}
        disabled={disabled}
        title="Target language"
        onChange={(e) =>
          onChange({
            ...value,
            // remember the previous target so swap-in-auto can toggle back
            altTargetLang:
              e.target.value === value.targetLang ? value.altTargetLang : value.targetLang,
            targetLang: e.target.value,
          })
        }
      >
        {LANGUAGES.map((l) => (
          <option key={l.code} value={l.code}>
            {l.label}
          </option>
        ))}
      </select>
    </div>
  );
}
