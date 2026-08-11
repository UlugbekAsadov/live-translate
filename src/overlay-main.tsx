import React from "react";
import ReactDOM from "react-dom/client";
import { OverlayApp } from "./windows/overlay/OverlayApp";
import { initEventBus } from "./services/eventBus";
import { useSettingsStore } from "./stores/settingsStore";
import "./styles.css";

(async () => {
  await initEventBus();
  await useSettingsStore.getState().init();
})().catch(console.error);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <OverlayApp />
  </React.StrictMode>,
);
