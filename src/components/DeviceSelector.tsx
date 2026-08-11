import { useEffect, useState } from "react";
import { ipc } from "../services/ipc";
import type { Device, Source } from "../types/ipc";

export function DeviceSelector({
  source,
  value,
  onChange,
  disabled,
}: {
  source: Source;
  value: string | null;
  onChange: (deviceId: string | null) => void;
  disabled?: boolean;
}) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    ipc
      .listAudioDevices()
      .then((list) => {
        if (!alive) return;
        // System audio is captured via loopback on an OUTPUT device.
        setDevices(source === "system" ? list.outputs : list.inputs);
      })
      .catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
  }, [source]);

  if (error) {
    return <p className="text-xs text-red-400">Audio devices unavailable: {error}</p>;
  }

  return (
    <select
      className="w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-slate-200 outline-none focus:border-emerald-400/50 disabled:opacity-50"
      value={value ?? ""}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}
    >
      <option value="">Default device</option>
      {devices.map((d) => (
        <option key={d.id} value={d.id}>
          {d.name}
          {d.isDefault ? " (default)" : ""}
        </option>
      ))}
    </select>
  );
}
