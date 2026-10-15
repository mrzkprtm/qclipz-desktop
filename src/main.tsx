import React from "react";
import ReactDOM from "react-dom/client";
import "./theme.css";
import App from "./App";
import Selector, { type OverlayConfig } from "./Selector";
import { Border, RecBar, type RecBarConfig } from "./Chrome";

// Helper windows (overlay, recording bar, border) get a config injected by Rust.
type Injected = ({ view: "overlay" } & OverlayConfig) | ({ view: "recbar" } & RecBarConfig) | { view: "border" };
const cfg = (window as { __QCLIPZ__?: Injected }).__QCLIPZ__;

function View() {
  if (cfg?.view === "overlay") return <Selector cfg={cfg} />;
  if (cfg?.view === "recbar") return <RecBar cfg={cfg} />;
  if (cfg?.view === "border") return <Border />;
  return <App />;
}

if (cfg) document.documentElement.classList.add("helper");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <View />
  </React.StrictMode>,
);
