use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};

mod audio;
mod win;

// A recording in progress: ffmpeg writes video only; audio tracks are captured in-process.
struct Session {
    child: Child,
    out: PathBuf,
    video: PathBuf,
    tracks: Vec<audio::Track>,
    boost: bool,
}

#[derive(Default)]
struct Recorder {
    active: Mutex<Option<Session>>,
    // Last finished recording; the only file `discard_recording` may delete.
    last: Mutex<Option<PathBuf>>,
}

fn log_path() -> PathBuf {
    std::env::temp_dir().join("qclipz-ffmpeg.log")
}

// Last few lines of the ffmpeg log, so errors say why ffmpeg failed.
fn log_tail() -> String {
    let text = std::fs::read_to_string(log_path()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(4)..].join("\n")
}

// ffmpeg without a console window popping up in release builds. Prefers the
// copy bundled next to the app (installer sidecar), then whatever is on PATH.
fn ffmpeg() -> Command {
    let bundled = std::env::current_exe().ok().and_then(|p| Some(p.parent()?.join("ffmpeg.exe")));
    let mut cmd = Command::new(bundled.filter(|p| p.exists()).unwrap_or_else(|| "ffmpeg".into()));
    cmd.arg("-hide_banner");
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

fn spawn_err(e: std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::NotFound {
        "ffmpeg not found. Install it (winget install Gyan.FFmpeg) and restart QClipz.".into()
    } else {
        format!("spawn ffmpeg: {}", e)
    }
}

// Run ffmpeg to completion; Ok(stderr) on success, Err with its last lines otherwise.
fn run_ffmpeg(args: &[String], what: &str) -> Result<String, String> {
    let out = ffmpeg().args(args).stdin(Stdio::null()).output().map_err(spawn_err)?;
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    if !out.status.success() {
        let tail: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).rev().take(4).collect();
        return Err(format!("{what} failed:\n{}", tail.into_iter().rev().collect::<Vec<_>>().join("\n")));
    }
    Ok(err)
}

async fn sleep(d: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    fn inside(&self, o: &win::Rect) -> bool {
        self.x >= o.left
            && self.y >= o.top
            && self.x + self.width as i32 <= o.right
            && self.y + self.height as i32 <= o.bottom
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordParams {
    output_path: String,
    fps: u32,
    area: Rect,
    // Record this window's on-screen rectangle instead of `area` ("0x1a2b").
    window_hwnd: Option<String>,
    // Microphone (WASAPI input device name).
    audio_device: Option<String>,
    // Whatever plays on the default speakers or headphones (WASAPI loopback).
    system_audio: bool,
    // Raise quiet audio towards a normal loudness when saving.
    #[serde(default)]
    boost_audio: bool,
    // Scale so the shorter side is this many px (1080 => 1920x1080 or 1080x1920).
    output_short_side: Option<u32>,
    draw_mouse: bool,
    show_border: bool,
    // Hide the main window while recording.
    #[serde(default)]
    hide_main: bool,
}

fn video_filter(short_side: Option<u32>) -> String {
    // libx264 + yuv420p needs even dimensions.
    let mut vf = String::from("crop=trunc(iw/2)*2:trunc(ih/2)*2");
    if let Some(n) = short_side.filter(|n| *n >= 2) {
        let n = n & !1;
        vf += &format!(",scale='if(gt(iw,ih),-2,{n})':'if(gt(iw,ih),{n},-2)':flags=bicubic");
    }
    vf + ",format=yuv420p"
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Backend {
    // Desktop Duplication: fast, tear-free, sees GPU content. One monitor at a time.
    Dda { output: usize, ox: i32, oy: i32 },
    // GDI: slow and can tear, but can span monitors.
    Gdi,
}

fn pick_backend(area: &Rect, outputs: &[win::Rect]) -> Backend {
    match outputs.iter().position(|o| area.inside(o)) {
        Some(i) => Backend::Dda { output: i, ox: area.x - outputs[i].left, oy: area.y - outputs[i].top },
        None => Backend::Gdi,
    }
}

// Video-only capture. Frames are stamped with wall-clock time since `epoch_us`, the same
// moment the audio tracks count from, so the mux at stop can line them up exactly.
// (Audio stays out of this process: live audio inputs make ffmpeg pace the screen grab
// to them, which dropped capture to ~5 fps on slower PCs.)
fn record_args(b: Backend, p: &RecordParams, area: &Rect, video: &Path, epoch_us: i64) -> Vec<String> {
    let mut a: Vec<String> = vec!["-y".into()];
    let clock = format!("setpts=(RTCTIME-{epoch_us})/(TB*1000000)");
    // fast_bilinear for the BGRA -> YUV conversion: ~12% less CPU, no visible loss on screen content.
    let swscale = "sws_flags=fast_bilinear;";
    let size = format!("{}x{}", area.width, area.height);
    let mouse = if p.draw_mouse { "1" } else { "0" };
    let vf = video_filter(p.output_short_side);

    let graph = match b {
        Backend::Dda { output, ox, oy } => format!(
            "ddagrab=output_idx={output}:framerate={fps}:draw_mouse={mouse}:offset_x={ox}:offset_y={oy}:video_size={size},{clock},hwdownload,format=bgra,{vf}[v]",
            fps = p.fps
        ),
        Backend::Gdi => {
            a.extend(["-f", "gdigrab", "-framerate"].map(String::from));
            a.push(p.fps.to_string());
            a.extend(["-draw_mouse".into(), mouse.into()]);
            a.extend(["-offset_x".into(), area.x.to_string(), "-offset_y".into(), area.y.to_string()]);
            a.extend(["-video_size".into(), size, "-i".into(), "desktop".into()]);
            format!("[0:v]{clock},{vf}[v]")
        }
    };
    a.extend(["-filter_complex".into(), format!("{swscale}{graph}"), "-map".into(), "[v]".into()]);
    // zerolatency drops x264's frame-thread buffering: ~14% less CPU here, steadier pacing.
    a.extend(["-c:v", "libx264", "-preset", "ultrafast", "-tune", "zerolatency", "-crf", "23"].map(String::from));
    a.push(video.to_string_lossy().into_owned());
    a
}

// Start ffmpeg; Err if it dies during startup (capture API failure...).
async fn spawn_recorder(args: &[String]) -> Result<Child, String> {
    let log_file = std::fs::File::create(log_path()).map_err(|e| format!("open log: {}", e))?;
    let log_file_err = log_file.try_clone().map_err(|e| format!("clone log: {}", e))?;
    let mut child = ffmpeg()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .map_err(spawn_err)?;
    // Keep the user's apps (the thing being recorded) responsive on slow PCs.
    win::lower_priority(&child);
    sleep(Duration::from_millis(900)).await;
    if let Ok(Some(_)) = child.try_wait() {
        return Err(format!("ffmpeg failed to start:\n{}", log_tail()));
    }
    Ok(child)
}

// Temporary files next to the final recording, removed after the mux.
fn part(out: &Path, suffix: &str) -> PathBuf {
    let mut name = out.file_stem().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    out.with_file_name(name)
}

fn now_us() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

fn discard_tracks(tracks: Vec<audio::Track>) {
    let paths: Vec<PathBuf> = tracks.iter().map(|t| t.path().clone()).collect();
    drop(tracks); // stops capture
    for f in paths {
        let _ = std::fs::remove_file(f);
    }
}

// Start audio tracks and the video ffmpeg for `area`. Returns the session and the backend used.
async fn begin(p: &RecordParams, area: &Rect, backend: Backend) -> Result<(Session, Backend), String> {
    let out = PathBuf::from(&p.output_path);
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {}", dir.display(), e))?;
    }
    // Shared timeline: audio byte 0 and video pts 0 are this moment.
    let start = Instant::now();
    let epoch_us = now_us();
    let mut tracks = Vec::new();
    if p.system_audio {
        tracks.push(audio::Track::start(&audio::Source::System, part(&out, "system.raw"), start)?);
    }
    if let Some(mic) = p.audio_device.as_deref().filter(|d| !d.is_empty()) {
        match audio::Track::start(&audio::Source::Mic(mic.into()), part(&out, "mic.raw"), start) {
            Ok(t) => tracks.push(t),
            Err(e) => {
                discard_tracks(tracks);
                return Err(e);
            }
        }
    }

    let video = part(&out, "part.mp4");
    let spawned = match spawn_recorder(&record_args(backend, p, area, &video, epoch_us)).await {
        Ok(c) => Ok((c, backend)),
        // Desktop Duplication can be unavailable (remote desktop, old drivers): fall back to GDI.
        Err(_) if backend != Backend::Gdi => spawn_recorder(&record_args(Backend::Gdi, p, area, &video, epoch_us))
            .await
            .map(|c| (c, Backend::Gdi)),
        Err(e) => Err(e),
    };
    match spawned {
        Ok((child, used)) => Ok((Session { child, out, video, tracks, boost: p.boost_audio }, used)),
        Err(e) => {
            discard_tracks(tracks);
            let _ = std::fs::remove_file(&video);
            Err(e)
        }
    }
}

// "start: 0.633000" from ffmpeg's input summary: where the video begins on the shared timeline.
fn video_start(video: &Path) -> f64 {
    let info = ffmpeg().arg("-i").arg(video).stdin(Stdio::null()).output();
    info.ok()
        .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
        .and_then(|s| s.split("start: ").nth(1)?.split(',').next()?.trim().parse().ok())
        .unwrap_or(0.0)
}

// Stop capture, then write the final mp4: video copied as-is, audio trimmed to start
// with the first video frame, mixed, optionally boosted, and encoded to AAC.
fn end(mut s: Session) -> Result<PathBuf, String> {
    let video_done = finish(s.child);
    for t in &mut s.tracks {
        t.stop()?;
    }
    video_done?;
    if !s.video.exists() {
        return Err(format!("recording file was not created:\n{}", log_tail()));
    }

    let mut args: Vec<String> = vec!["-y".into(), "-i".into(), s.video.to_string_lossy().into_owned()];
    if !s.tracks.is_empty() {
        let offset = video_start(&s.video);
        let mut audio_inputs = Vec::new();
        let mut chains = Vec::new();
        for (i, t) in s.tracks.iter().enumerate() {
            audio_inputs.extend(t.ffmpeg_input());
            chains.push(format!("[{i}:a]atrim=start={offset:.6},asetpts=PTS-STARTPTS[a{i}]"));
        }
        let mix = if s.tracks.len() == 1 {
            "[a0]anull".to_string()
        } else {
            "[a0][a1]amix=inputs=2:duration=longest:normalize=0".to_string()
        };
        // Audio-only graph, inputs numbered from 0.
        let mut graph = format!("{};{mix}", chains.join(";"));
        if s.boost {
            graph += &boost_filter(&audio_inputs, &graph);
        }
        // In the mux the video is input 0, so shift the audio input numbers by one.
        for i in (0..s.tracks.len()).rev() {
            graph = graph.replace(&format!("[{i}:a]"), &format!("[{}:a]", i + 1));
        }
        args.extend(audio_inputs);
        args.extend(["-filter_complex".into(), format!("{graph}[a]")]);
        args.extend(["-map", "0:v", "-c:v", "copy", "-map", "[a]", "-c:a", "aac", "-b:a", "160k", "-ac", "2", "-shortest"].map(String::from));
    } else {
        args.extend(["-c", "copy"].map(String::from));
    }
    args.extend(["-movflags", "+faststart"].map(String::from));
    args.push(s.out.to_string_lossy().into_owned());
    run_ffmpeg(&args, "saving the recording")?;

    let _ = std::fs::remove_file(&s.video);
    for t in &s.tracks {
        let _ = std::fs::remove_file(t.path());
    }
    Ok(s.out.clone())
}

// Measure the mix's loudness and return a gain stage that brings quiet audio up to about
// -16 LUFS (a typical level for online video). Only ever boosts, capped at +24 dB, with a
// limiter so peaks don't clip.
fn boost_filter(audio_inputs: &[String], graph: &str) -> String {
    let mut args: Vec<String> = vec!["-nostats".into()];
    args.extend(audio_inputs.iter().cloned());
    args.extend(["-filter_complex".into(), format!("{graph},ebur128=framelog=quiet"), "-f".into(), "null".into(), "-".into()]);
    let lufs = run_ffmpeg(&args, "measuring loudness").ok().and_then(|err| {
        let summary = err.rsplit("Summary:").next()?.to_string();
        summary.split("I:").nth(1)?.split("LUFS").next()?.trim().parse::<f64>().ok()
    });
    match lufs {
        Some(i) if i > -70.0 => {
            let gain = (-16.0 - i).clamp(0.0, 24.0);
            if gain < 0.5 { String::new() } else { format!(",volume={gain:.1}dB,alimiter=limit=0.95:level=0") }
        }
        _ => String::new(), // silence or unknown: leave as recorded
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Started {
    area: Rect,
    backend: &'static str,
}

#[tauri::command]
async fn start_record(
    app: tauri::AppHandle,
    state: tauri::State<'_, Recorder>,
    params: RecordParams,
) -> Result<Started, String> {
    if state.active.lock().unwrap().is_some() {
        return Err("a recording is already running".into());
    }
    if !(1..=120).contains(&params.fps) {
        return Err("fps must be between 1 and 120".into());
    }

    let mut area = params.area;
    if let Some(hwnd) = params.window_hwnd.as_deref().filter(|t| !t.is_empty()) {
        let h = isize::from_str_radix(hwnd.trim_start_matches("0x"), 16)
            .map_err(|_| format!("bad window handle {hwnd}"))?;
        win::bring_to_front(h)?;
        sleep(Duration::from_millis(350)).await;
        let r = win::window_bounds(h).ok_or("couldn't read the window's position")?;
        area = Rect { x: r.left, y: r.top, width: (r.right - r.left) as u32, height: (r.bottom - r.top) as u32 };
    }
    area.width &= !1;
    area.height &= !1;
    if area.width < 16 || area.height < 16 {
        return Err("the area must be at least 16×16".into());
    }

    // Windows that span two screens get clipped to the screen holding most of them.
    let outputs = win::dxgi_outputs();
    if params.window_hwnd.is_some()
        && pick_backend(&area, &outputs) == Backend::Gdi
        && let Some(o) = outputs.iter().max_by_key(|o| overlap(&area, o))
    {
        area = clip(&area, o);
    }

    let (session, used) = begin(&params, &area, pick_backend(&area, &outputs)).await?;
    *state.active.lock().unwrap() = Some(session);
    // The floating controls and border are hidden from capture, so they never show up in the video.
    let _ = open_recording_chrome(&app, &area, params.show_border);
    if params.hide_main
        && let Some(main) = app.get_webview_window("main")
    {
        let _ = main.hide();
    }
    Ok(Started { area, backend: if used == Backend::Gdi { "gdi" } else { "dda" } })
}

fn overlap(a: &Rect, o: &win::Rect) -> i64 {
    let w = (a.x + a.width as i32).min(o.right) - a.x.max(o.left);
    let h = (a.y + a.height as i32).min(o.bottom) - a.y.max(o.top);
    if w > 0 && h > 0 { w as i64 * h as i64 } else { 0 }
}

fn clip(a: &Rect, o: &win::Rect) -> Rect {
    let x = a.x.max(o.left);
    let y = a.y.max(o.top);
    let r = (a.x + a.width as i32).min(o.right);
    let b = (a.y + a.height as i32).min(o.bottom);
    Rect { x, y, width: (r - x).max(0) as u32, height: (b - y).max(0) as u32 }
}

fn exclude(w: &tauri::WebviewWindow) {
    // Set QCLIPZ_CAPTURE_SELF=1 to keep QClipz visible to screenshots (testing, or recording QClipz itself).
    if std::env::var_os("QCLIPZ_CAPTURE_SELF").is_some() {
        return;
    }
    if let Ok(h) = w.hwnd() {
        win::exclude_from_capture(h.0 as isize);
    }
}

fn helper_window<'a>(
    app: &'a tauri::AppHandle,
    label: &str,
    cfg: serde_json::Value,
) -> tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle> {
    tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("index.html".into()))
        .initialization_script(format!("window.__QCLIPZ__ = {cfg};"))
        .title("QClipz")
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
}

const BAR_W: f64 = 440.0;
const BORDER: i32 = 3;

// The recording bar and border are WebView windows; creating one starts a browser
// renderer, which on a slow PC stalls everything for a second. So they're built once,
// hidden, when QClipz starts, and recording only moves and shows them.
fn recbar(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window("recbar") {
        return Ok(w);
    }
    let w = helper_window(app, "recbar", serde_json::json!({ "view": "recbar", "startedAt": 0, "area": { "width": 0, "height": 0 } }))
        .inner_size(BAR_W, 56.0)
        .transparent(true)
        .focused(false)
        .focusable(false) // clicks still work; showing it never takes focus from a game or video
        .build()?;
    exclude(&w);
    Ok(w)
}

fn border(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window("border") {
        return Ok(w);
    }
    // Opaque and clipped to a ring (see open_recording_chrome), not a transparent overlay.
    let w = helper_window(app, "border", serde_json::json!({ "view": "border" }))
        .focused(false)
        .focusable(false)
        .build()?;
    exclude(&w);
    w.set_ignore_cursor_events(true)?;
    Ok(w)
}

fn open_recording_chrome(app: &tauri::AppHandle, area: &Rect, show_border: bool) -> tauri::Result<()> {
    if show_border {
        let w = border(app)?;
        let (bw, bh) = (area.width + 2 * BORDER as u32, area.height + 2 * BORDER as u32);
        w.set_position(tauri::PhysicalPosition::new(area.x - BORDER, area.y - BORDER))?;
        w.set_size(tauri::PhysicalSize::new(bw, bh))?;
        if let Ok(h) = w.hwnd() {
            win::ring_region(h.0 as isize, bw as i32, bh as i32, BORDER);
        }
        w.show()?;
    }

    // Floating stop bar, top-center of the screen being recorded.
    let monitors = app.available_monitors()?;
    let (cx, cy) = (area.x + area.width as i32 / 2, area.y + area.height as i32 / 2);
    let m = monitors
        .iter()
        .find(|m| {
            let (p, s) = (m.position(), m.size());
            cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
        })
        .or(monitors.first());
    let bar = recbar(app)?;
    if let Some(m) = m {
        let scale = m.scale_factor();
        bar.set_size(tauri::LogicalSize::new(BAR_W, 56.0))?;
        bar.set_position(tauri::PhysicalPosition::new(
            m.position().x + (m.size().width as i32 - (BAR_W * scale) as i32) / 2,
            m.position().y + (16.0 * scale) as i32,
        ))?;
    }
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let _ = app.emit_to("recbar", "recbar-start", serde_json::json!({ "startedAt": started, "area": area }));
    bar.show()?;
    Ok(())
}

fn close_recording_chrome(app: &tauri::AppHandle) {
    for label in ["recbar", "border"] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.hide();
        }
    }
}

// 'q' tells ffmpeg to finalize the mp4 cleanly; then wait for it.
fn finish(mut child: Child) -> Result<(), String> {
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"q\n");
    }
    let status = child.wait().map_err(|e| format!("wait ffmpeg: {}", e))?;
    if !status.success() {
        return Err(format!("ffmpeg exited with {:?}:\n{}", status.code(), log_tail()));
    }
    Ok(())
}

#[tauri::command]
async fn stop_record(app: tauri::AppHandle, state: tauri::State<'_, Recorder>) -> Result<String, String> {
    close_recording_chrome(&app);
    let session = state.active.lock().unwrap().take().ok_or("no recording in progress")?;
    let out = tauri::async_runtime::spawn_blocking(move || end(session))
        .await
        .map_err(|e| e.to_string())??;
    // Let the webview load this one file through the asset protocol for preview.
    app.asset_protocol_scope().allow_file(&out).map_err(|e| e.to_string())?;
    *state.last.lock().unwrap() = Some(out.clone());
    Ok(out.to_string_lossy().into_owned())
}

#[tauri::command]
fn discard_recording(state: tauri::State<'_, Recorder>) -> Result<(), String> {
    if let Some(path) = state.last.lock().unwrap().take() {
        win::recycle(&path)?;
    }
    Ok(())
}

// Re-encode with a crop filter, save to chosen path.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CropParams {
    input_path: String,
    output_path: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[tauri::command]
async fn crop_and_save(app: tauri::AppHandle, params: CropParams) -> Result<(), String> {
    if params.input_path == params.output_path {
        return Err("choose a different file name for the cropped video".into());
    }
    let (w, h) = (params.width & !1, params.height & !1);
    if w < 2 || h < 2 {
        return Err("crop area is too small".into());
    }
    let filter = format!("crop={}:{}:{}:{}", w, h, params.x, params.y);
    let out = params.output_path.clone();
    let output = tauri::async_runtime::spawn_blocking(move || {
        ffmpeg()
            .args([
                "-y",
                "-i", &params.input_path,
                "-vf", &filter,
                "-c:v", "libx264",
                "-preset", "veryfast",
                "-pix_fmt", "yuv420p",
                "-c:a", "copy",
                &params.output_path,
            ])
            .stdin(Stdio::null())
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(spawn_err)?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = err.lines().rev().take(4).collect();
        return Err(format!("crop failed:\n{}", tail.into_iter().rev().collect::<Vec<_>>().join("\n")));
    }
    let _ = app.asset_protocol_scope().allow_file(&out);
    Ok(())
}

// Microphones and other capture devices (WASAPI), default first.
#[tauri::command]
fn list_audio_devices() -> Vec<String> {
    audio::input_names()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowInfo {
    hwnd: String,
    title: String,
    exe: String,
    minimized: bool,
}

#[tauri::command]
fn system_audio_device() -> Option<String> {
    audio::output_name()
}

#[tauri::command]
fn list_windows() -> Vec<WindowInfo> {
    let mut list: Vec<WindowInfo> = win::list_windows()
        .into_iter()
        .map(|w| WindowInfo { hwnd: format!("0x{:x}", w.hwnd), title: w.title, exe: w.exe, minimized: w.minimized })
        .collect();
    list.sort_by_key(|w| w.title.to_lowercase());
    list
}

// One still of the whole virtual desktop, for the screen map in the main window.
#[tauri::command]
async fn snapshot_desktop(app: tauri::AppHandle) -> Result<String, String> {
    // One at a time, so a newer snapshot never deletes the file an older reply points at.
    static LOCK: Mutex<()> = Mutex::new(());
    let path = tauri::async_runtime::spawn_blocking(|| -> Result<PathBuf, String> {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join("qclipz-snapshots");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("desktop-{stamp}.jpg"));
        let output = ffmpeg()
            .args(["-y", "-f", "gdigrab", "-i", "desktop", "-frames:v", "1", "-vf", "scale=960:-2", "-q:v", "4"])
            .arg(&path)
            .stdin(Stdio::null())
            .output()
            .map_err(spawn_err)?;
        if !output.status.success() || !path.exists() {
            return Err("couldn't take a screen snapshot".into());
        }
        for old in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            if old.path() != path {
                let _ = std::fs::remove_file(old.path());
            }
        }
        Ok(path)
    })
    .await
    .map_err(|e| e.to_string())??;
    app.asset_protocol_scope().allow_file(&path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

// Open a transparent drag-to-select overlay on every monitor. The overlay calls
// `finish_selection`, which hides them all and sends the area to main. Overlays are kept
// (hidden) and reused while the monitors don't change, so closing one never tears down a
// browser renderer right as a recording starts.
#[tauri::command]
async fn open_selector(app: tauri::AppHandle, aspect: Option<f64>, current: Option<Rect>) -> Result<(), String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let mut overlays = overlay_windows(&app);
    let same_layout = overlays.len() == monitors.len()
        && overlays.iter().zip(&monitors).all(|(w, m)| {
            w.outer_position().ok() == Some(*m.position()) && w.outer_size().ok() == Some(*m.size())
        });
    let cfg = |m: &tauri::Monitor| {
        serde_json::json!({
            "view": "overlay",
            "originX": m.position().x,
            "originY": m.position().y,
            "aspect": aspect,
            "current": current,
        })
    };

    if same_layout {
        for (w, m) in overlays.iter().zip(&monitors) {
            let _ = app.emit_to(w.label(), "selector-config", cfg(m));
        }
        sleep(Duration::from_millis(60)).await; // let them re-render before showing
    } else {
        for w in overlays.drain(..) {
            let _ = w.destroy();
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        for (i, m) in monitors.iter().enumerate() {
            let w = helper_window(&app, &format!("overlay-{stamp}-{i}"), cfg(m))
                .transparent(true)
                .build()
                .map_err(|e| e.to_string())?;
            // Physical units so mixed-DPI monitors line up exactly.
            w.set_position(*m.position()).map_err(|e| e.to_string())?;
            w.set_size(*m.size()).map_err(|e| e.to_string())?;
            overlays.push(w);
        }
    }
    for w in &overlays {
        w.show().map_err(|e| e.to_string())?;
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    if let Some(w) = overlays.first() {
        let _ = w.set_focus();
    }
    // Cover the taskbar too, so areas that include it can be framed.
    for w in &overlays {
        if let Ok(h) = w.hwnd() {
            win::raise_topmost(h.0 as isize);
        }
    }
    Ok(())
}

// Overlay windows in monitor order.
fn overlay_windows(app: &tauri::AppHandle) -> Vec<tauri::WebviewWindow> {
    let mut list: Vec<(usize, tauri::WebviewWindow)> = app
        .webview_windows()
        .into_iter()
        .filter_map(|(label, w)| {
            let i = label.strip_prefix("overlay-")?.rsplit('-').next()?.parse().ok()?;
            Some((i, w))
        })
        .collect();
    list.sort_by_key(|(i, _)| *i);
    list.into_iter().map(|(_, w)| w).collect()
}

fn close_overlays(app: &tauri::AppHandle) {
    for w in overlay_windows(app) {
        let _ = w.hide();
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Selection {
    area: Rect,
    aspect: Option<f64>,
    // Start recording right away instead of going back to the main window.
    record: bool,
}

#[tauri::command]
async fn finish_selection(app: tauri::AppHandle, selection: Option<Selection>) -> Result<(), String> {
    close_overlays(&app);
    let recording = selection.as_ref().is_some_and(|s| s.record);
    // When recording straight away the main window stays hidden (start_record hides it anyway).
    if !recording && let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
    if let Some(s) = selection {
        app.emit_to("main", "area-selected", s).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Open a recording or folder with its default app. Only folders and .mp4 files, so the UI
// can never be used to launch a program.
#[tauri::command]
fn open_in_shell(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("{} doesn't exist", p.display()));
    }
    if !(p.is_dir() || p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4"))) {
        return Err("only folders and .mp4 recordings can be opened".into());
    }
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn show_main(app: tauri::AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Launching QClipz again focuses the running window instead of starting a second recorder.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app.clone())))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Recorder::default())
        .setup(|app| {
            // QClipz never records itself.
            if let Some(main) = app.get_webview_window("main") {
                exclude(&main);
            }
            let handle = app.handle();
            recbar(handle)?;
            border(handle)?;
            Ok(())
        })
        .on_window_event(|w, e| {
            // Closing the main window quits, even if helper windows are open.
            if w.label() == "main" && matches!(e, tauri::WindowEvent::Destroyed) {
                w.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_record,
            stop_record,
            discard_recording,
            crop_and_save,
            list_audio_devices,
            system_audio_device,
            list_windows,
            snapshot_desktop,
            open_selector,
            finish_selection,
            open_in_shell,
            show_main
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Don't leave ffmpeg running (and the mp4 unfinalized) when the app closes.
            if let tauri::RunEvent::Exit = event
                && let Some(session) = app.state::<Recorder>().active.lock().unwrap().take()
            {
                let _ = end(session);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_dda_only_inside_one_monitor() {
        let outs = [
            win::Rect { left: 0, top: 0, right: 1920, bottom: 1080 },
            win::Rect { left: 1920, top: 159, right: 3286, bottom: 927 },
        ];
        let on2 = Rect { x: 2221, y: 259, width: 280, height: 500 };
        assert_eq!(pick_backend(&on2, &outs), Backend::Dda { output: 1, ox: 301, oy: 100 });
        let spans = Rect { x: 1800, y: 200, width: 400, height: 300 };
        assert_eq!(pick_backend(&spans, &outs), Backend::Gdi);
        assert_eq!(clip(&spans, &outs[0]), Rect { x: 1800, y: 200, width: 120, height: 300 });
    }
}

#[cfg(test)]
mod probe {
    // cargo test print_outputs -- --ignored --nocapture
    #[test]
    #[ignore]
    fn print_outputs() {
        for (i, r) in super::win::dxgi_outputs().iter().enumerate() {
            println!("output {i}: ({}, {}) - ({}, {})", r.left, r.top, r.right, r.bottom);
        }
    }
}

#[cfg(test)]
mod sync_probe {
    use super::*;
    // Records through the app path (no UI) to measure A/V sync and fps:
    //   QZ_AREA=x,y,w,h QZ_SECS=11 [QZ_MIC=name] [QZ_BOOST=1] cargo test sync_probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn sync_probe() {
        let out = std::env::temp_dir().join("qz-sync-rec.mp4");
        let n: Vec<i32> = std::env::var("QZ_AREA")
            .unwrap_or("1920,159,640,360".into())
            .split(',')
            .map(|v| v.parse().unwrap())
            .collect();
        let area = Rect { x: n[0], y: n[1], width: n[2] as u32, height: n[3] as u32 };
        let p = RecordParams {
            output_path: out.to_string_lossy().into_owned(),
            fps: 30,
            area,
            window_hwnd: None,
            audio_device: std::env::var("QZ_MIC").ok(),
            system_audio: true,
            boost_audio: std::env::var("QZ_BOOST").is_ok(),
            output_short_side: None,
            draw_mouse: false,
            show_border: false,
            hide_main: false,
        };
        let backend = pick_backend(&area, &win::dxgi_outputs());
        let (session, used) = tauri::async_runtime::block_on(begin(&p, &area, backend)).unwrap();
        std::thread::sleep(Duration::from_secs(std::env::var("QZ_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(11)));
        let path = end(session).unwrap();
        println!("backend {used:?} -> {}", path.display());
    }
}
