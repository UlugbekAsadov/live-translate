import { useEffect, useState } from "react";
import { Dashboard } from "./pages/Dashboard";
import { Settings } from "./pages/Settings";
import { initEventBus } from "./services/eventBus";
import { handleShortcut } from "./services/controller";
import { useSettingsStore } from "./stores/settingsStore";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { ipc } from "./services/ipc";
import type { ShortcutActionPayload } from "./types/ipc";

type Tab = "dashboard" | "settings";

export function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const loaded = useSettingsStore((s) => s.loaded);

  useEffect(() => {
    (async () => {
      await initEventBus();
      await useSettingsStore.getState().init();
      const { settings } = useSettingsStore.getState();
      // Register global shortcuts from persisted settings at startup.
      try {
        await ipc.applyShortcuts(settings.shortcuts);
      } catch (e) {
        console.error("failed to register shortcuts", e);
      }
    })();
  }, []);

  useTauriEvent<ShortcutActionPayload>("shortcut:action", (p) => {
    handleShortcut(p.action).catch(console.error);
  });

  if (!loaded) return null;

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between border-b border-white/10 px-4 py-2.5">
        <h1 className="text-sm font-semibold tracking-wide text-slate-200">
          AI Meeting Translator
        </h1>
        <nav className="flex gap-1 rounded-lg bg-white/5 p-0.5">
          {(["dashboard", "settings"] as Tab[]).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => setTab(t)}
              className={`rounded-md px-3 py-1 text-xs font-medium capitalize ${
                tab === t ? "bg-white/10 text-slate-100" : "text-slate-400 hover:text-slate-200"
              }`}
            >
              {t}
            </button>
          ))}
        </nav>
      </header>
      <main className="min-h-0 flex-1 overflow-hidden">
        {tab === "dashboard" ? <Dashboard /> : <Settings />}
      </main>
    </div>
  );
}
