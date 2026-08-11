import { useEffect, useState } from "react";
import { ipc } from "../services/ipc";

export function ApiKeyForm() {
  const [hasKey, setHasKey] = useState(false);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    ipc.hasApiKey().then(setHasKey).catch(console.error);
  }, []);

  async function save() {
    setBusy(true);
    setMessage(null);
    try {
      await ipc.setApiKey(draft.trim());
      setDraft("");
      setHasKey(true);
      setMessage({ ok: true, text: "API key saved to Windows Credential Manager." });
    } catch (e) {
      setMessage({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  async function test() {
    setBusy(true);
    setMessage(null);
    try {
      const r = await ipc.testApiKey();
      setMessage(
        r.ok
          ? { ok: true, text: "API key is valid." }
          : { ok: false, text: r.error ?? "API key check failed." },
      );
    } catch (e) {
      setMessage({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    setBusy(true);
    setMessage(null);
    try {
      await ipc.deleteApiKey();
      setHasKey(false);
      setMessage({ ok: true, text: "API key removed." });
    } catch (e) {
      setMessage({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2">
        <input
          type="password"
          className="flex-1 rounded-md border border-white/10 bg-white/5 px-3 py-1.5 text-sm text-slate-200 outline-none placeholder:text-slate-500 focus:border-emerald-400/50"
          placeholder={hasKey ? "•••••••• (a key is saved)" : "sk-…"}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          autoComplete="off"
        />
        <button
          type="button"
          disabled={busy || draft.trim().length < 8}
          onClick={save}
          className="rounded-md bg-emerald-500 px-3 py-1.5 text-sm font-medium text-emerald-950 hover:bg-emerald-400 disabled:opacity-40"
        >
          Save
        </button>
        <button
          type="button"
          disabled={busy || !hasKey}
          onClick={test}
          className="rounded-md border border-white/10 bg-white/5 px-3 py-1.5 text-sm text-slate-200 hover:bg-white/10 disabled:opacity-40"
        >
          Test
        </button>
        {hasKey && (
          <button
            type="button"
            disabled={busy}
            onClick={remove}
            className="rounded-md border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-sm text-red-300 hover:bg-red-500/20 disabled:opacity-40"
          >
            Delete
          </button>
        )}
      </div>
      {message && (
        <p className={`text-xs ${message.ok ? "text-emerald-400" : "text-red-400"}`}>
          {message.text}
        </p>
      )}
      <p className="text-xs text-slate-500">
        Stored securely in Windows Credential Manager — never in a config file and never
        exposed to this window after saving.
      </p>
    </div>
  );
}
