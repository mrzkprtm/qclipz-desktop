import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import "./theme.css";
import App from "./App";
import Selector, { type OverlayConfig } from "./Selector";
import { Border, RecBar, type RecBarConfig } from "./Chrome";

// Helper windows (overlay, recording bar, border) get a config injected by Rust.
type Injected = ({ view: "overlay" } & OverlayConfig) | ({ view: "recbar" } & RecBarConfig) | { view: "border" };
const cfg = (window as { __QCLIPZ__?: Injected }).__QCLIPZ__;

// Overlays are reused between selections; a new config remounts the selector fresh.
function OverlayHost({ initial }: { initial: OverlayConfig }) {
  const [current, setCurrent] = useState({ cfg: initial, key: 0 });
  useEffect(() => {
    const un = listen<OverlayConfig>("selector-config", (e) => setCurrent((c) => ({ cfg: e.payload, key: c.key + 1 })));
    return () => {
      un.then((f) => f());
    };
  }, []);
  return <Selector key={current.key} cfg={current.cfg} />;
}

function View() {
  if (cfg?.view === "overlay") return <OverlayHost initial={cfg} />;
  if (cfg?.view === "recbar") return <RecBar cfg={cfg} />;
  if (cfg?.view === "border") return <Border />;
  return <App />;
}

if (cfg) document.documentElement.classList.add("helper", `view-${cfg.view}`);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <View />
  </React.StrictMode>,
);
