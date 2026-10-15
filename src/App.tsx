import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { videoDir } from "@tauri-apps/api/path";
import { availableMonitors } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ASPECTS } from "./Selector";
import { timecode } from "./Chrome";
import {
  IconArea, IconCheck, IconClose, IconCrop, IconFolder, IconMic, IconOpen, IconPlay, IconSpeaker,
  IconRefresh, IconScreen, IconSearch, IconStop, IconTrash, IconWindow,
} from "./icons";
import "./App.css";

type Mode = "screen" | "area" | "window";
type Phase = "idle" | "starting" | "recording" | "stopping" | "review" | "saving";

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
interface Screen extends Rect {
  name: string;
  main: boolean;
}
interface WindowInfo {
  hwnd: string;
  title: string;
  exe: string;
  minimized: boolean;
}
interface Problem {
  title: string;
  detail?: string;
}

// Shorter side of the output video; 0 keeps the captured size.
const RESOLUTIONS = [0, 2160, 1440, 1080, 720, 480];
const FPS = [24, 30, 60];
const even = (n: number) => Math.max(2, Math.floor(n / 2) * 2);
const round2 = (n: number) => Math.round(n / 2) * 2; // matches ffmpeg scale=-2

function outputSize(w: number, h: number, short: number) {
  if (!short) return `${w & ~1} × ${h & ~1}`;
  return w > h ? `${round2((w * short) / h)} × ${short}` : `${short} × ${round2((h * short) / w)}`;
}

function fitAspect(r: Rect, aspect: number): Rect {
  const width = aspect >= 1 ? r.width : even(r.height * aspect);
  const height = aspect >= 1 ? even(r.width / aspect) : r.height;
  return { x: Math.round(r.x + (r.width - width) / 2), y: Math.round(r.y + (r.height - height) / 2), width, height };
}

// A saved area that no longer fits any screen (a monitor was unplugged) is shrunk to
// fit the main screen and centered on it.
function keepOnScreen(r: Rect, screens: Screen[]): Rect {
  const fits = (s: Screen) => r.x >= s.x && r.y >= s.y && r.x + r.width <= s.x + s.width && r.y + r.height <= s.y + s.height;
  if (!screens.length || screens.some(fits)) return r;
  const s = screens.find((m) => m.main) ?? screens[0];
  const width = even(Math.min(r.width, s.width));
  const height = even(Math.min(r.height, s.height));
  return { x: s.x + Math.floor((s.width - width) / 2), y: s.y + Math.floor((s.height - height) / 2), width, height };
}

function stamp() {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}-${p(d.getMinutes())}-${p(d.getSeconds())}`;
}

function problem(e: unknown): Problem {
  const [first, ...rest] = String(e).split("\n");
  return { title: first, detail: rest.join("\n") || undefined };
}

const fileName = (p: string) => p.split(/[\\/]/).pop() ?? p;

// Settings remember themselves between sessions (best effort).
function usePersisted<T>(key: string, initial: T): [T, (v: T | ((old: T) => T)) => void] {
  const [value, setValue] = useState<T>(() => {
    try {
      const raw = localStorage.getItem(`qclipz.${key}`);
      return raw === null ? initial : (JSON.parse(raw) as T);
    } catch {
      return initial;
    }
  });
  useEffect(() => {
    try {
      localStorage.setItem(`qclipz.${key}`, JSON.stringify(value));
    } catch {
      /* storage unavailable */
    }
  }, [key, value]);
  return [value, setValue];
}

export default function App() {
  const [screens, setScreens] = useState<Screen[]>([{ name: "Screen 1", main: true, x: 0, y: 0, width: 1920, height: 1080 }]);
  const [snapshot, setSnapshot] = useState<string | null>(null);
  const [snapping, setSnapping] = useState(false);

  const [mode, setMode] = usePersisted<Mode>("mode", "screen");
  const [screenIdx, setScreenIdx] = usePersisted("screen", 0); // -1 = all screens
  const [area, setArea] = usePersisted<Rect>("area", { x: 320, y: 180, width: 1280, height: 720 });
  const [aspect, setAspect] = usePersisted<number | null>("aspect", null);
  const [fps, setFps] = usePersisted("fps", 30);
  const [resolution, setResolution] = usePersisted("resolution", 0);
  const [drawMouse, setDrawMouse] = usePersisted("cursor", true);
  const [showBorder, setShowBorder] = usePersisted("border", true);
  const [hideWhileRecording, setHide] = usePersisted("hide", true);
  const [audioOn, setAudioOn] = usePersisted("audio", false); // microphone
  const [systemAudio, setSystemAudio] = usePersisted("systemAudio", false);
  const [boostAudio, setBoostAudio] = usePersisted("boostAudio", true);
  const [outputName, setOutputName] = useState<string | null>(null);
  const [audioDevice, setAudioDevice] = usePersisted("audioDevice", "");
  const [folder, setFolder] = usePersisted("folder", "");

  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [hwnd, setHwnd] = useState("");
  const [query, setQuery] = useState("");
  const [audioDevices, setAudioDevices] = useState<string[] | null>(null);

  const [phase, setPhase] = useState<Phase>("idle");
  const [startedAt, setStartedAt] = useState(0);
  const [now, setNow] = useState(Date.now());
  const [recArea, setRecArea] = useState<Rect | null>(null);
  const [outputPath, setOutputPath] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [error, setError] = useState<Problem | null>(null);

  const [cropping, setCropping] = useState(false);
  const [crop, setCrop] = useState<Rect | null>(null);
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [videoInfo, setVideoInfo] = useState<{ w: number; h: number; duration: number } | null>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  const bounds: Rect = useMemo(() => {
    const x = Math.min(...screens.map((s) => s.x));
    const y = Math.min(...screens.map((s) => s.y));
    return {
      x,
      y,
      width: Math.max(...screens.map((s) => s.x + s.width)) - x,
      height: Math.max(...screens.map((s) => s.y + s.height)) - y,
    };
  }, [screens]);
  const screenRect: Rect = screenIdx < 0 ? bounds : screens[screenIdx] ?? screens[0];
  const source: Rect = mode === "area" ? area : screenRect;
  const selectedWindow = windows.find((w) => w.hwnd === hwnd);
  const busy = phase !== "idle" && phase !== "review";
  const live = phase === "recording" || phase === "stopping";

  // Only the newest reply wins, so overlapping refreshes can't show a stale file.
  const snapSeq = useRef(0);
  function loadScreens() {
    availableMonitors()
      .then((ms) => {
        if (!ms.length) return;
        const list = ms.map((m, i) => ({
          name: `Screen ${i + 1}`,
          main: m.position.x === 0 && m.position.y === 0,
          x: m.position.x,
          y: m.position.y,
          width: m.size.width,
          height: m.size.height,
        }));
        setScreens(list);
        setScreenIdx((i) => (i < list.length ? i : 0));
        setArea((r) => keepOnScreen(r, list));
      })
      .catch(() => {}); // not running inside Tauri
  }

  async function takeSnapshot() {
    loadScreens(); // monitors may have been plugged in or out
    const seq = ++snapSeq.current;
    setSnapping(true);
    try {
      const path = await invoke<string>("snapshot_desktop");
      if (seq === snapSeq.current) setSnapshot(convertFileSrc(path));
    } catch {
      if (seq === snapSeq.current) setSnapshot(null);
    } finally {
      if (seq === snapSeq.current) setSnapping(false);
    }
  }

  async function refreshWindows() {
    try {
      const list = await invoke<WindowInfo[]>("list_windows");
      setWindows(list);
      setHwnd((h) => (list.some((w) => w.hwnd === h) ? h : list[0]?.hwnd ?? ""));
    } catch (e) {
      setError(problem(e));
    }
  }

  async function loadAudioDevices() {
    try {
      const list = await invoke<string[]>("list_audio_devices");
      setAudioDevices(list);
      setAudioDevice((d) => (list.includes(d) ? d : list[0] ?? ""));
    } catch (e) {
      setError(problem(e));
    }
  }

  // Latest actions for listeners that are registered once.
  const actions = useRef({ record: (_?: { area: Rect }) => {}, stop: () => {} });

  useEffect(() => {
    takeSnapshot();
    if (!folder) videoDir().then((d) => setFolder(`${d.replace(/[\\/]$/, "")}\\QClipz`)).catch(() => {});

    const unSel = listen<{ area: Rect; aspect: number | null; record: boolean }>("area-selected", (e) => {
      setArea(e.payload.area);
      setAspect(e.payload.aspect);
      setMode("area");
      if (e.payload.record) actions.current.record({ area: e.payload.area });
    });
    const unBar = listen<string>("recbar", (e) => {
      if (e.payload === "stop") actions.current.stop();
    });
    return () => {
      unSel.then((f) => f()).catch(() => {});
      unBar.then((f) => f()).catch(() => {});
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (mode === "window") refreshWindows();
  }, [mode]);

  useEffect(() => {
    if (audioOn && !audioDevices) loadAudioDevices();
    if (systemAudio) invoke<string | null>("system_audio_device").then(setOutputName).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [audioOn, systemAudio]);

  useEffect(() => {
    if (phase !== "recording") return;
    const id = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(id);
  }, [phase]);

  async function record(override?: { area: Rect }) {
    setError(null);
    setNote(null);
    const m: Mode = override ? "area" : mode;
    const src = override?.area ?? source;
    if (m === "window" && !hwnd) return setError({ title: "Pick a window to record." });
    if (m !== "window" && (src.width < 16 || src.height < 16))
      return setError({ title: "The area is too small. Make it at least 16 × 16." });
    if (audioOn && !audioDevice)
      return setError({ title: "No microphone selected.", detail: "Pick one under Audio, or turn Microphone off." });
    if (!folder) return setError({ title: "Choose a folder to save recordings in." });

    setPhase("starting");
    try {
      const started = await invoke<{ area: Rect; backend: string }>("start_record", {
        params: {
          outputPath: `${folder}\\QClipz ${stamp()}.mp4`,
          fps,
          area: src,
          windowHwnd: m === "window" ? hwnd : null,
          audioDevice: audioOn ? audioDevice : null,
          systemAudio,
          boostAudio,
          outputShortSide: resolution || null,
          drawMouse,
          showBorder,
          hideMain: hideWhileRecording || !!override,
        },
      });
      setRecArea(started.area);
      setStartedAt(Date.now());
      setNow(Date.now());
      setPhase("recording");
    } catch (e) {
      setPhase("idle");
      invoke("show_main");
      setError(problem(e));
    }
  }

  async function stop() {
    setPhase("stopping");
    try {
      const path = await invoke<string>("stop_record");
      setOutputPath(path);
      setVideoInfo(null);
      setCropping(false);
      setCrop(null);
      setPhase("review");
    } catch (e) {
      setPhase("idle");
      setError(problem(e));
    }
    invoke("show_main");
  }

  actions.current = { record, stop };

  // Ctrl+R starts or stops from anywhere in the main window.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.ctrlKey && e.key.toLowerCase() === "r") {
        e.preventDefault();
        if (phase === "recording") stop();
        else if (phase === "idle" || phase === "review") record();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  async function selectOnScreen() {
    setError(null);
    try {
      await invoke("open_selector", { aspect, current: area });
    } catch (e) {
      setError(problem(e));
    }
  }

  function changeAspect(a: number | null) {
    setAspect(a);
    if (a) setArea((r) => fitAspect(r, a));
  }

  async function chooseFolder() {
    const dir = await open({ directory: true, defaultPath: folder || undefined, title: "Save recordings to" });
    if (typeof dir === "string") setFolder(dir);
  }

  // ----- review / crop -----
  function stagePoint(e: React.PointerEvent<HTMLDivElement>) {
    const r = e.currentTarget.getBoundingClientRect();
    return { x: Math.min(Math.max(e.clientX - r.left, 0), r.width), y: Math.min(Math.max(e.clientY - r.top, 0), r.height) };
  }

  async function saveCropped() {
    const video = videoRef.current;
    if (!outputPath || !video || !crop) return;
    if (crop.width < 8 || crop.height < 8) return setError({ title: "Drag on the video to draw the crop first." });
    const target = await save({
      title: "Save cropped copy",
      defaultPath: outputPath.replace(/\.mp4$/i, " (cropped).mp4"),
      filters: [{ name: "Video", extensions: ["mp4"] }],
    });
    if (!target) return;
    const scale = video.videoWidth / video.clientWidth;
    setPhase("saving");
    try {
      await invoke("crop_and_save", {
        params: {
          inputPath: outputPath,
          outputPath: target,
          x: Math.round(crop.x * scale),
          y: Math.round(crop.y * scale),
          width: Math.min(Math.round(crop.width * scale), video.videoWidth),
          height: Math.min(Math.round(crop.height * scale), video.videoHeight),
        },
      });
      setNote(target);
      setCropping(false);
      setCrop(null);
    } catch (e) {
      setError(problem(e));
    } finally {
      setPhase("review");
    }
  }

  async function discard() {
    try {
      await invoke("discard_recording");
      setOutputPath(null);
      setPhase("idle");
    } catch (e) {
      setError(problem(e));
    }
  }

  const q = query.trim().toLowerCase();
  const shownWindows = q
    ? windows.filter((w) => w.title.toLowerCase().includes(q) || w.exe.toLowerCase().includes(q))
    : windows;
  const appName = (w: WindowInfo) => w.exe.replace(/\.exe$/i, "") || "app";

  return (
    <div className="shell">
      <main className="work">
        <header className="topbar">
          <div className="brand">
            <span className={`lamp ${live ? "live" : phase === "idle" ? "armed" : ""}`} />
            QClipz
          </div>
          {phase !== "review" ? (
            <div className="seg tabs" role="tablist" aria-label="What to record">
              {([
                ["screen", "Full screen", IconScreen],
                ["area", "Area", IconArea],
                ["window", "Window", IconWindow],
              ] as const).map(([m, label, I]) => (
                <button key={m} role="tab" aria-pressed={mode === m} aria-selected={mode === m} disabled={busy}
                  onClick={() => { setMode(m); setError(null); }}>
                  <I size={16} /> {label}
                </button>
              ))}
            </div>
          ) : (
            <div className="review-title">
              <IconPlay size={16} /> <span title={outputPath ?? ""}>{outputPath && fileName(outputPath)}</span>
            </div>
          )}
          <div className="topbar-end">
            {mode !== "window" && phase === "idle" && (
              <button className="btn ghost" onClick={takeSnapshot} disabled={snapping} title="Refresh the screen preview">
                <IconRefresh size={16} /> {snapping ? "Refreshing…" : "Refresh preview"}
              </button>
            )}
            {mode === "window" && phase === "idle" && (
              <button className="btn ghost" onClick={refreshWindows}><IconRefresh size={16} /> Refresh list</button>
            )}
          </div>
        </header>

        {error && (
          <div className="banner" role="alert">
            <div>
              <strong>{error.title}</strong>
              {error.detail && (
                <details><summary>Details</summary><pre>{error.detail}</pre></details>
              )}
            </div>
            <button className="btn ghost icon" onClick={() => setError(null)} aria-label="Dismiss"><IconClose size={16} /></button>
          </div>
        )}

        {phase === "review" || phase === "saving" ? (
          outputPath && (
            <section className="monitor review">
              <div
                className={`stage ${cropping ? "cropping" : ""}`}
                onPointerDown={(e) => {
                  if (!cropping) return;
                  e.currentTarget.setPointerCapture(e.pointerId);
                  const p = stagePoint(e);
                  setDragStart(p);
                  setCrop({ x: p.x, y: p.y, width: 0, height: 0 });
                }}
                onPointerMove={(e) => {
                  if (!cropping || !dragStart) return;
                  const p = stagePoint(e);
                  setCrop({
                    x: Math.min(dragStart.x, p.x), y: Math.min(dragStart.y, p.y),
                    width: Math.abs(p.x - dragStart.x), height: Math.abs(p.y - dragStart.y),
                  });
                }}
                onPointerUp={() => setDragStart(null)}
              >
                <video
                  ref={videoRef}
                  src={convertFileSrc(outputPath)}
                  controls={!cropping}
                  autoPlay
                  onLoadedMetadata={(e) =>
                    setVideoInfo({ w: e.currentTarget.videoWidth, h: e.currentTarget.videoHeight, duration: e.currentTarget.duration })}
                />
                {cropping && crop && crop.width > 0 && (
                  <div className="crop-rect" style={{ left: crop.x, top: crop.y, width: crop.width, height: crop.height }}>
                    <i className="corner tl" /><i className="corner tr" /><i className="corner bl" /><i className="corner br" />
                  </div>
                )}
              </div>
            </section>
          )
        ) : mode === "window" ? (
          <section className={`monitor windows ${live ? "live" : ""}`}>
            <label className="search">
              <IconSearch size={16} />
              <input className="field" placeholder="Filter by app or title" value={query}
                onChange={(e) => setQuery(e.target.value)} disabled={busy} />
            </label>
            <div className="win-grid" role="listbox" aria-label="Windows">
              {shownWindows.map((w) => (
                <button key={w.hwnd} role="option" aria-selected={w.hwnd === hwnd} className="win-cell" disabled={busy}
                  onClick={() => setHwnd(w.hwnd)} onDoubleClick={() => record()}>
                  <span className="win-exe">
                    <span className={`lamp ${w.hwnd === hwnd ? (live ? "live" : "armed") : ""}`} />
                    {appName(w)}
                    {w.minimized && <em>minimized</em>}
                  </span>
                  <span className="win-title">{w.title}</span>
                </button>
              ))}
              {shownWindows.length === 0 && (
                <p className="empty">
                  {windows.length ? "No window matches that filter." : "No windows found. Open the app you want to record, then refresh the list."}
                </p>
              )}
            </div>
          </section>
        ) : (
          <ScreenMap
            screens={screens}
            bounds={bounds}
            snapshot={snapshot}
            selection={live && recArea ? recArea : source}
            live={live}
            mode={mode}
            disabled={busy}
            selectedScreen={screenIdx}
            onPickScreen={setScreenIdx}
            onDrawArea={(r) => setArea(aspect ? fitAspect(r, aspect) : r)}
            aspectControl={
              <div className="seg" role="group" aria-label="Aspect ratio">
                {ASPECTS.map((a) => (
                  <button key={a.label} disabled={busy} onClick={() => changeAspect(a.value)}
                    aria-pressed={aspect === a.value || (!!aspect && !!a.value && Math.abs(aspect - a.value) < 0.01)}>
                    {a.label}
                  </button>
                ))}
              </div>
            }
          />
        )}

        <footer className="strip">
          {phase === "review" || phase === "saving" ? (
            <>
              <span className="meta tnum">
                {videoInfo ? `${videoInfo.w} × ${videoInfo.h} · ${timecode(videoInfo.duration * 1000)}` : "Loading…"}
              </span>
              {note && (
                <span className="saved">
                  <IconCheck size={16} /> Cropped copy saved
                  <button className="link" onClick={() => revealItemInDir(note)}>Show</button>
                </span>
              )}
              <div className="strip-end">
                {cropping ? (
                  <>
                    <span className="hint">Drag on the video to frame the crop</span>
                    <button className="btn" onClick={() => { setCropping(false); setCrop(null); }}>Cancel</button>
                    <button className="btn" onClick={saveCropped} disabled={!crop || crop.width < 8 || phase === "saving"}>
                      <IconCheck size={16} /> {phase === "saving" ? "Saving…" : "Save cropped copy"}
                    </button>
                  </>
                ) : (
                  <>
                    <button className="btn" onClick={() => setCropping(true)}><IconCrop size={16} /> Crop</button>
                    <button className="btn" onClick={() => outputPath && invoke("open_in_shell", { path: outputPath }).catch((e) => setError(problem(e)))}><IconOpen size={16} /> Open</button>
                    <button className="btn" onClick={() => outputPath && revealItemInDir(outputPath)}><IconFolder size={16} /> Show in folder</button>
                    <button className="btn danger" onClick={discard} title="Moves the file to the Recycle Bin"><IconTrash size={16} /> Delete</button>
                    <button className="btn" onClick={() => { setPhase("idle"); setNote(null); takeSnapshot(); }}>Back to setup</button>
                  </>
                )}
              </div>
            </>
          ) : mode === "area" ? (
            <>
              <button className="btn" onClick={selectOnScreen} disabled={busy}><IconArea size={16} /> Select on screen</button>
              <div className="xywh tnum">
                {(["x", "y", "width", "height"] as const).map((k) => (
                  <label key={k}>
                    <span>{k === "width" ? "W" : k === "height" ? "H" : k.toUpperCase()}</span>
                    <input className="field" type="number" value={area[k]} disabled={busy}
                      onChange={(e) => setArea((r) => ({ ...r, [k]: Math.round(+e.target.value) }))} />
                  </label>
                ))}
              </div>
            </>
          ) : mode === "window" ? (
            <span className="meta">
              {selectedWindow ? <>Records <b>{appName(selectedWindow)}</b> where it sits on screen. It's brought to the front when recording starts.</> : "Pick a window."}
            </span>
          ) : (
            <span className="meta tnum">Captures {source.width} × {source.height} at {source.x}, {source.y}. Click a screen to switch.</span>
          )}
        </footer>
      </main>

      <aside className="rail">
        <fieldset disabled={busy} className="rail-body">
          <h2>Output</h2>
          <label className="row">
            <span>Resolution</span>
            <select className="field" value={resolution} onChange={(e) => setResolution(+e.target.value)}>
              {RESOLUTIONS.map((r) => (
                <option key={r} value={r}>{r ? `${r}p` : "Original"}</option>
              ))}
            </select>
          </label>
          <div className="row">
            <span>Frame rate</span>
            <div className="seg" role="group" aria-label="Frame rate">
              {FPS.map((f) => (
                <button key={f} aria-pressed={fps === f} onClick={() => setFps(f)}>{f}</button>
              ))}
            </div>
          </div>
          {fps === 60 && mode !== "window" && source.width * source.height > 1280 * 720 && (
            <p className="advice">60 fps above 720p can drop frames on this PC. 30 fps records smoother.</p>
          )}
          <label className="switch"><span>Show cursor</span><input type="checkbox" checked={drawMouse} onChange={(e) => setDrawMouse(e.target.checked)} /><span className="track" /></label>
          <label className="switch"><span>Red border while recording</span><input type="checkbox" checked={showBorder} onChange={(e) => setShowBorder(e.target.checked)} /><span className="track" /></label>
          <label className="switch"><span>Hide QClipz while recording</span><input type="checkbox" checked={hideWhileRecording} onChange={(e) => setHide(e.target.checked)} /><span className="track" /></label>

          <h2>Audio</h2>
          <label className="switch">
            <span className="with-icon"><IconSpeaker size={16} /> System sound</span>
            <input type="checkbox" checked={systemAudio} onChange={(e) => setSystemAudio(e.target.checked)} /><span className="track" />
          </label>
          {systemAudio && (
            <p className="note">
              Records what plays on {outputName ? <b>{outputName}</b> : "your default speakers or headphones"}. No Stereo Mix needed.
            </p>
          )}
          <label className="switch"><span className="with-icon"><IconMic size={16} /> Microphone</span><input type="checkbox" checked={audioOn} onChange={(e) => setAudioOn(e.target.checked)} /><span className="track" /></label>
          {audioOn && (
            audioDevices === null ? <p className="advice">Looking for devices…</p> :
            audioDevices.length ? (
              <select className="field" value={audioDevice} onChange={(e) => setAudioDevice(e.target.value)} aria-label="Audio device">
                {audioDevices.map((d) => (
                  <option key={d} value={d}>{d}</option>
                ))}
              </select>
            ) : (
              <p className="advice">No microphones found. Plug one in, then turn this off and on again.</p>
            )
          )}
          {(systemAudio || audioOn) && (
            <label className="switch">
              <span>Boost quiet audio</span>
              <input type="checkbox" checked={boostAudio} onChange={(e) => setBoostAudio(e.target.checked)} /><span className="track" />
            </label>
          )}

          <h2>Save to</h2>
          <div className="folder">
            <span className="path" title={folder}>{folder || "Not set"}</span>
            <button className="btn" onClick={chooseFolder}>Change</button>
            <button className="btn icon" title="Open folder" aria-label="Open folder"
              onClick={() => invoke("open_in_shell", { path: folder }).catch(() => setError({ title: "That folder doesn't exist yet. It's created with your first recording." }))}>
              <IconFolder size={16} />
            </button>
          </div>
        </fieldset>

        <div className="rail-foot">
          <p className="summary tnum">
            {live && recArea
              ? <>Recording {recArea.width} × {recArea.height} · {fps} fps</>
              : <>{phase === "review" ? "Next: " : ""}{mode === "screen" ? (screenIdx < 0 ? "All screens" : screens[screenIdx]?.name ?? "Screen") : mode === "area" ? "Area" : "Window"} · </>}
            {live ? null : mode === "window"
                ? <>{selectedWindow ? appName(selectedWindow) : "No window"} → {resolution ? `${resolution}p` : "own size"} · {fps} fps</>
                : <>{source.width} × {source.height} → {outputSize(source.width, source.height, resolution)} · {fps} fps</>}
          </p>
          <button
            className={`onair ${live ? "lit" : ""}`}
            onClick={() => (live ? stop() : record())}
            disabled={phase === "starting" || phase === "stopping" || phase === "saving"}
            aria-label={live ? "Stop recording" : "Start recording"}
          >
            <span className="onair-letters tnum">{live ? timecode(now - startedAt) : "ON AIR"}</span>
            <span className="onair-sub">
              {phase === "starting" ? "Starting…" : phase === "stopping" ? "Saving…" : live
                ? <><IconStop size={12} /> Stop <kbd>Ctrl R</kbd></>
                : <>Record <kbd>Ctrl R</kbd></>}
            </span>
          </button>
        </div>
      </aside>
    </div>
  );
}

function ScreenMap(props: {
  screens: Screen[];
  bounds: Rect;
  snapshot: string | null;
  selection: Rect;
  live: boolean;
  mode: Mode;
  disabled: boolean;
  selectedScreen: number;
  onPickScreen: (i: number) => void;
  onDrawArea: (r: Rect) => void;
  aspectControl: React.ReactNode;
}) {
  const { screens, bounds: b, snapshot, selection: s, live, mode, disabled } = props;
  const [draw, setDraw] = useState<{ x: number; y: number; cur: { x: number; y: number } } | null>(null);
  const pct = (r: Rect) => ({
    left: `${((r.x - b.x) / b.width) * 100}%`,
    top: `${((r.y - b.y) / b.height) * 100}%`,
    width: `${(r.width / b.width) * 100}%`,
    height: `${(r.height / b.height) * 100}%`,
  });
  // Pointer → physical desktop pixels.
  function phys(e: React.PointerEvent<HTMLDivElement>) {
    const r = e.currentTarget.getBoundingClientRect();
    const fx = Math.min(Math.max((e.clientX - r.left) / r.width, 0), 1);
    const fy = Math.min(Math.max((e.clientY - r.top) / r.height, 0), 1);
    return { x: Math.round(b.x + fx * b.width), y: Math.round(b.y + fy * b.height) };
  }
  const drawn = draw && {
    x: Math.min(draw.x, draw.cur.x),
    y: Math.min(draw.y, draw.cur.y),
    width: even(Math.abs(draw.cur.x - draw.x)),
    height: even(Math.abs(draw.cur.y - draw.y)),
  };
  const shown = drawn && drawn.width > 4 ? drawn : s;
  const drawable = mode === "area" && !disabled;

  return (
    <section className={`monitor ${live ? "live" : ""}`}>
      <div className="map-fit">
        <div
          className={`map ${drawable ? "drawable" : ""}`}
          style={{ ["--ratio" as string]: b.width / b.height }}
          onPointerDown={(e) => {
            if (!drawable || e.button !== 0) return;
            e.currentTarget.setPointerCapture(e.pointerId);
            const p = phys(e);
            setDraw({ ...p, cur: p });
          }}
          onPointerMove={(e) => draw && setDraw({ ...draw, cur: phys(e) })}
          onPointerUp={() => {
            if (drawn && drawn.width >= 16 && drawn.height >= 16) props.onDrawArea(drawn);
            setDraw(null);
          }}
        >
          {snapshot && <img className="snap" src={snapshot} alt="" draggable={false} onError={(e) => (e.currentTarget.style.visibility = "hidden")} onLoad={(e) => (e.currentTarget.style.visibility = "")} />}
          {screens.map((m, i) => (
            <button
              key={i}
              className={`screen ${mode === "screen" && (props.selectedScreen === i || props.selectedScreen < 0) ? "on" : ""}`}
              style={pct(m)}
              disabled={disabled || mode !== "screen"}
              onClick={() => props.onPickScreen(i)}
              aria-label={`${m.name}, ${m.width} by ${m.height}`}
            >
              <span className="screen-tag tnum">
                {i + 1}
                <small>{m.width}×{m.height}{m.main ? " · main" : ""}</small>
              </span>
            </button>
          ))}
          <div className={`frame ${live ? "live" : ""}`} style={pct(shown)}>
            <i className="corner tl" /><i className="corner tr" /><i className="corner bl" /><i className="corner br" />
            <span className="frame-tag tnum">{live ? "LIVE · " : ""}{shown.width} × {shown.height}</span>
          </div>
        </div>
      </div>
      {mode === "screen" && screens.length > 1 && (
        <div className="map-foot">
          <div className="seg" role="group" aria-label="Screens">
            {screens.map((m, i) => (
              <button key={i} aria-pressed={props.selectedScreen === i} disabled={disabled} onClick={() => props.onPickScreen(i)}>
                <span className="lamp" /> {m.name}
              </button>
            ))}
            <button aria-pressed={props.selectedScreen < 0} disabled={disabled} onClick={() => props.onPickScreen(-1)}>
              <span className="lamp" /> All screens
            </button>
          </div>
        </div>
      )}
      {mode === "area" && (
        <div className="map-foot">
          {props.aspectControl}
          {drawable && <span className="map-hint">Drag here for a rough area, or Select on screen for pixel-exact framing.</span>}
        </div>
      )}
    </section>
  );
}
