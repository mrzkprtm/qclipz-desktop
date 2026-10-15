// Windows shown while recording. Both are excluded from capture by Rust,
// so they never appear in the video.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import { IconGrip, IconRestore, IconStop } from "./icons";
import "./Chrome.css";

export interface RecBarConfig {
  startedAt: number; // ms since epoch
  area: { width: number; height: number };
}

export function timecode(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(Math.floor(s / 3600))}:${pad(Math.floor(s / 60) % 60)}:${pad(s % 60)}`;
}

export function RecBar({ cfg }: { cfg: RecBarConfig }) {
  const [now, setNow] = useState(Date.now());
  const [stopping, setStopping] = useState(false);

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(id);
  }, []);

  function stop() {
    setStopping(true);
    emitTo("main", "recbar", "stop");
  }

  return (
    <div className="recbar" data-tauri-drag-region>
      <span className="recbar-grip" data-tauri-drag-region title="Drag to move">
        <IconGrip size={16} />
      </span>
      <span className="recbar-sign" data-tauri-drag-region>
        <span className="recbar-onair" data-tauri-drag-region>ON AIR</span>
        <span className="recbar-time tnum" data-tauri-drag-region>{timecode(now - cfg.startedAt)}</span>
      </span>
      <span className="recbar-size tnum" data-tauri-drag-region>
        {cfg.area.width}×{cfg.area.height}
      </span>
      <button className="recbar-btn" title="Show QClipz" aria-label="Show QClipz" onClick={() => invoke("show_main")}>
        <IconRestore size={16} />
      </button>
      <button className="recbar-stop" onClick={stop} disabled={stopping}>
        <IconStop size={14} />
        {stopping ? "Saving…" : "Stop"}
      </button>
    </div>
  );
}

export function Border() {
  return <div className="rec-border" />;
}
