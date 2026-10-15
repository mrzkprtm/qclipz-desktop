import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { IconCheck, IconClose, IconExpand, IconRecord } from "./icons";
import "./Selector.css";

// Injected by the `open_selector` command into each overlay window.
export interface OverlayConfig {
  originX: number; // monitor top-left, physical px
  originY: number;
  aspect: number | null; // width / height, null = free
  current: { x: number; y: number; width: number; height: number } | null;
}

interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
type Drag =
  | { kind: "draw"; ax: number; ay: number }
  | { kind: "move"; sx: number; sy: number; start: Box }
  | { kind: "edge"; handle: Handle; sx: number; sy: number; start: Box };

const MIN = 8; // css px

export const ASPECTS: { label: string; value: number | null }[] = [
  { label: "Free", value: null },
  { label: "16:9", value: 16 / 9 },
  { label: "9:16", value: 9 / 16 },
  { label: "1:1", value: 1 },
  { label: "4:3", value: 4 / 3 },
];
// Physical pixel sizes people actually post at.
const PRESETS = [
  { label: "1920 × 1080 · YouTube", w: 1920, h: 1080 },
  { label: "1280 × 720 · HD", w: 1280, h: 720 },
  { label: "1080 × 1920 · Shorts / Reels", w: 1080, h: 1920 },
  { label: "1080 × 1080 · Square", w: 1080, h: 1080 },
  { label: "1080 × 1350 · Portrait post", w: 1080, h: 1350 },
];

const sameAspect = (a: number | null, b: number | null) =>
  a === b || (a !== null && b !== null && Math.abs(a - b) < 0.01);

// Rectangle from anchor (ax, ay) to pointer (px, py), kept on screen and at `aspect`.
function fromAnchor(ax: number, ay: number, px: number, py: number, aspect: number | null): Box {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const dx = px - ax;
  const dy = py - ay;
  let w = Math.abs(dx);
  let h = Math.abs(dy);
  if (aspect) {
    if (w / Math.max(h, 1) > aspect) w = h * aspect;
    else h = w / aspect;
  }
  const maxW = dx < 0 ? ax : vw - ax;
  const maxH = dy < 0 ? ay : vh - ay;
  const k = Math.min(1, maxW / Math.max(w, 1), maxH / Math.max(h, 1));
  w *= k;
  h *= k;
  return { x: dx < 0 ? ax - w : ax, y: dy < 0 ? ay - h : ay, w, h };
}

// Box of size w×h centered on (cx, cy), shrunk to fit the screen and clamped inside it.
function centered(cx: number, cy: number, w: number, h: number): Box {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const k = Math.min(1, vw / w, vh / h);
  w *= k;
  h *= k;
  return {
    x: Math.min(Math.max(cx - w / 2, 0), vw - w),
    y: Math.min(Math.max(cy - h / 2, 0), vh - h),
    w,
    h,
  };
}

export default function Selector({ cfg }: { cfg: OverlayConfig }) {
  const dpr = window.devicePixelRatio || 1;
  const label = getCurrentWindow().label;
  const rootRef = useRef<HTMLDivElement>(null);
  const [aspect, setAspect] = useState<number | null>(cfg.aspect);

  // Start with the current area if it's on this monitor.
  const [box, setBox] = useState<Box | null>(() => {
    const c = cfg.current;
    if (!c || c.width < 2 || c.height < 2) return null;
    const x = (c.x - cfg.originX) / dpr;
    const y = (c.y - cfg.originY) / dpr;
    if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight) return null;
    return { x, y, w: Math.min(c.width / dpr, window.innerWidth - x), h: Math.min(c.height / dpr, window.innerHeight - y) };
  });
  const [drag, setDrag] = useState<Drag | null>(null);

  // Physical, even-sized area in virtual-desktop coordinates.
  const toArea = (b: Box) => ({
    x: cfg.originX + Math.round(b.x * dpr),
    y: cfg.originY + Math.round(b.y * dpr),
    width: Math.floor((b.w * dpr) / 2) * 2,
    height: Math.floor((b.h * dpr) / 2) * 2,
  });

  const finish = (record: boolean) =>
    box && box.w >= MIN && box.h >= MIN &&
    invoke("finish_selection", { selection: { area: toArea(box), aspect, record } });
  const cancel = () => invoke("finish_selection", { selection: null });

  // Only one monitor holds a selection at a time.
  function claim() {
    emit("selector-start", label);
  }

  useEffect(() => {
    rootRef.current?.focus();
    const un = listen<string>("selector-start", (e) => {
      if (e.payload !== label) setBox(null);
    });
    return () => {
      un.then((f) => f());
    };
  }, [label]);

  function pickAspect(a: number | null) {
    setAspect(a);
    if (a && box) {
      // Reshape around the box's center, keeping its larger side.
      const w = a >= 1 ? box.w : box.h * a;
      const h = a >= 1 ? box.w / a : box.h;
      setBox(centered(box.x + box.w / 2, box.y + box.h / 2, w, h));
    }
  }

  function pickPreset(i: number) {
    const p = PRESETS[i];
    if (!p) return;
    claim();
    setAspect(p.w / p.h);
    const cx = box ? box.x + box.w / 2 : window.innerWidth / 2;
    const cy = box ? box.y + box.h / 2 : window.innerHeight / 2;
    setBox(centered(cx, cy, p.w / dpr, p.h / dpr));
  }

  function wholeScreen() {
    claim();
    setAspect(null);
    setBox({ x: 0, y: 0, w: window.innerWidth, h: window.innerHeight });
  }

  function onKey(e: React.KeyboardEvent) {
    if (e.target instanceof HTMLSelectElement) return;
    const k = e.key.toLowerCase();
    if (k === "escape") cancel();
    else if (k === "enter") finish(false);
    else if (k === "r") finish(true);
    else if (k === "f") wholeScreen();
    else if (k >= "1" && k <= "5") pickAspect(ASPECTS[+k - 1].value);
    else if (box && e.key.startsWith("Arrow")) {
      // Nudge 1 px (Shift: 10 px); with Ctrl, resize instead of move.
      e.preventDefault();
      const step = (e.shiftKey ? 10 : 1) / dpr;
      const dx = e.key === "ArrowLeft" ? -step : e.key === "ArrowRight" ? step : 0;
      const dy = e.key === "ArrowUp" ? -step : e.key === "ArrowDown" ? step : 0;
      setBox((b) =>
        b && (e.ctrlKey
          ? { ...b, w: Math.max(MIN, b.w + dx), h: Math.max(MIN, b.h + dy) }
          : { ...b, x: b.x + dx, y: b.y + dy }),
      );
    }
  }

  function begin(e: React.PointerEvent, d: Drag) {
    e.preventDefault();
    e.stopPropagation();
    rootRef.current?.setPointerCapture(e.pointerId);
    setDrag(d);
  }

  function onDown(e: React.PointerEvent) {
    if (e.button !== 0) return;
    claim();
    setBox(null);
    begin(e, { kind: "draw", ax: e.clientX, ay: e.clientY });
  }

  function startHandle(e: React.PointerEvent, handle: Handle) {
    if (!box) return;
    // Corners drag from the opposite corner, which also keeps the aspect lock.
    const opp = {
      x: handle.includes("w") ? box.x + box.w : box.x,
      y: handle.includes("n") ? box.y + box.h : box.y,
    };
    if (handle.length === 2) begin(e, { kind: "draw", ax: opp.x, ay: opp.y });
    else begin(e, { kind: "edge", handle, sx: e.clientX, sy: e.clientY, start: box });
  }

  function onMove(e: React.PointerEvent) {
    if (!drag) return;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const px = Math.min(Math.max(e.clientX, 0), vw);
    const py = Math.min(Math.max(e.clientY, 0), vh);

    if (drag.kind === "draw") {
      setBox(fromAnchor(drag.ax, drag.ay, px, py, aspect));
    } else if (drag.kind === "move") {
      const s = drag.start;
      setBox({
        ...s,
        x: Math.min(Math.max(s.x + px - drag.sx, 0), vw - s.w),
        y: Math.min(Math.max(s.y + py - drag.sy, 0), vh - s.h),
      });
    } else {
      const s = drag.start;
      let { x, y, w, h } = s;
      if (drag.handle === "e") w = Math.max(MIN, s.w + px - drag.sx);
      if (drag.handle === "s") h = Math.max(MIN, s.h + py - drag.sy);
      if (drag.handle === "w") {
        x = Math.min(px, s.x + s.w - MIN);
        w = s.x + s.w - x;
      }
      if (drag.handle === "n") {
        y = Math.min(py, s.y + s.h - MIN);
        h = s.y + s.h - y;
      }
      setBox({ x, y, w: Math.min(w, vw - x), h: Math.min(h, vh - y) });
    }
  }

  function onUp() {
    setDrag(null);
    if (box && (box.w < MIN || box.h < MIN)) setBox(null);
  }

  const area = box && toArea(box);
  const handles: Handle[] = aspect ? ["nw", "ne", "sw", "se"] : ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
  // Action bar under the box, above it, or inside it — wherever there's room.
  const barPos = !box ? "" : box.y + box.h + 60 < window.innerHeight ? "below" : box.y > 60 ? "above" : "inside";
  const stop = (e: React.SyntheticEvent) => e.stopPropagation();

  return (
    <div
      ref={rootRef}
      className={`sel-root ${box ? "has-box" : ""} ${drag ? "dragging" : ""}`}
      tabIndex={0}
      onKeyDown={onKey}
      onPointerDown={onDown}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onContextMenu={(e) => {
        e.preventDefault();
        cancel();
      }}
    >
      {!drag && (
        <div className="sel-toolbar" onPointerDown={stop} onDoubleClick={stop}>
          <div className="seg" role="group" aria-label="Aspect ratio">
            {ASPECTS.map((a, i) => (
              <button key={a.label} aria-pressed={sameAspect(aspect, a.value)} onClick={() => pickAspect(a.value)}
                title={`Aspect ${a.label} (${i + 1})`}>
                {a.label}
              </button>
            ))}
          </div>
          <select className="field" value="" onChange={(e) => pickPreset(+e.target.value)} aria-label="Size preset">
            <option value="" disabled>Exact size…</option>
            {PRESETS.map((p, i) => (
              <option key={p.label} value={i}>{p.label}</option>
            ))}
          </select>
          <button className="btn" onClick={wholeScreen} title="Whole screen (F)">
            <IconExpand size={16} /> Whole screen
          </button>
          <span className="sel-sep" />
          <button className="btn ghost icon" onClick={cancel} title="Cancel (Esc)" aria-label="Cancel">
            <IconClose size={16} />
          </button>
        </div>
      )}

      {!box && !drag && (
        <p className="sel-hint">
          Drag anywhere to frame what you want to record
          <span>Right-click or Esc to cancel</span>
        </p>
      )}

      {box && area && (
        <div
          className="sel-box"
          style={{ left: box.x, top: box.y, width: box.w, height: box.h }}
          onPointerDown={(e) => e.button === 0 && begin(e, { kind: "move", sx: e.clientX, sy: e.clientY, start: box })}
          onDoubleClick={() => finish(false)}
        >
          <i className="corner tl" /><i className="corner tr" /><i className="corner bl" /><i className="corner br" />
          <span className={`sel-size tnum ${box.y < 30 ? "in" : ""}`}>
            {area.width} × {area.height}
          </span>
          {!drag &&
            handles.map((h) => (
              <span key={h} className={`sel-handle h-${h}`} onPointerDown={(e) => e.button === 0 && startHandle(e, h)} />
            ))}
          {!drag && (
            <div className={`sel-actions ${barPos}`} onPointerDown={stop} onDoubleClick={stop}>
              <button className="btn live" onClick={() => finish(true)}>
                <IconRecord size={16} /> Record <kbd>R</kbd>
              </button>
              <button className="btn" onClick={() => finish(false)}>
                <IconCheck size={16} /> Use area <kbd>Enter</kbd>
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
