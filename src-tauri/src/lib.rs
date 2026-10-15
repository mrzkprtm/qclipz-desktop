use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{Emitter, Manager};

mod loopback;
mod win;

#[derive(Default)]
struct Recorder {
    active: Mutex<Option<(Child, PathBuf, Option<loopback::Loopback>)>>,
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
    // Microphone / DirectShow device.
    audio_device: Option<String>,
    // Whatever plays on the default speakers or headphones (WASAPI loopback).
    system_audio: bool,
    // Scale so the shorter side is this many px (1080 => 1920x1080 or 1080x1920).
    output_short_side: Option<u32>,
    draw_mouse: bool,
    show_border: bool,
}

fn video_filter(short_side: Option<u32>) -> String {
    // libx264 + yuv420p needs even dimensions.
    let mut vf = String::from("crop=trunc(iw/2)*2:trunc(ih/2)*2");
    if let Some(n) = short_side.filter(|n| *n >= 2) {
        let n = n & !1;
        vf += &format!(",scale='if(gt(iw,ih),-2,{n})':'if(gt(iw,ih),{n},-2)':flags=lanczos");
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

fn record_args(b: Backend, p: &RecordParams, area: &Rect, system: Option<&loopback::Source>) -> Vec<String> {
    // One clock for every stream: inputs keep their own timestamps (-copyts) and the graph
    // below moves video and audio onto wall-clock time since the graph started (RTCSTART).
    // Without this, audio counts from when ffmpeg opened it and video from its first frame,
    // so audio ends up late by ffmpeg's startup time.
    let mut a: Vec<String> = vec!["-y".into(), "-copyts".into()];
    let mut inputs = 0;
    let wallclock_video = "setpts=(RTCTIME-RTCSTART)/(TB*1000000)";
    let size = format!("{}x{}", area.width, area.height);
    let mouse = if p.draw_mouse { "1" } else { "0" };
    let vf = video_filter(p.output_short_side);

    let video = match b {
        Backend::Dda { output, ox, oy } => format!(
            "ddagrab=output_idx={output}:framerate={fps}:draw_mouse={mouse}:offset_x={ox}:offset_y={oy}:video_size={size},{wallclock_video},hwdownload,format=bgra,{vf}[v]",
            fps = p.fps
        ),
        Backend::Gdi => {
            a.extend(["-f", "gdigrab", "-framerate"].map(String::from));
            a.push(p.fps.to_string());
            a.extend(["-draw_mouse".into(), mouse.into()]);
            a.extend(["-offset_x".into(), area.x.to_string(), "-offset_y".into(), area.y.to_string()]);
            a.extend(["-video_size".into(), size, "-i".into(), "desktop".into()]);
            inputs += 1;
            format!("[0:v]{wallclock_video},{vf}[v]")
        }
    };

    // System sound: sample 0 is the feeder's start moment, known on the wall clock.
    let sys = system.map(|src| {
        a.extend(src.ffmpeg_input());
        inputs += 1;
        format!(
            "[{i}:a]asetpts=PTS-(RTCSTART-{epoch})/(TB*1000000),atrim=start=0",
            i = inputs - 1,
            epoch = src.epoch_us()
        )
    });
    // Microphone: stamped with the wall clock on arrival, with a small buffer to keep that close to capture.
    let mic = p.audio_device.as_deref().filter(|d| !d.is_empty()).map(|dev| {
        a.extend(["-thread_queue_size", "1024", "-use_wallclock_as_timestamps", "1", "-audio_buffer_size", "50", "-f", "dshow", "-i"].map(String::from));
        a.push(format!("audio={dev}"));
        inputs += 1;
        format!(
            "[{i}:a]asetpts=PTS-RTCSTART/(TB*1000000),atrim=start=0,aresample=async=1000:first_pts=0",
            i = inputs - 1
        )
    });
    let audio = match (sys, mic) {
        (Some(s), Some(m)) => Some(format!("{s}[sys];{m}[mic];[sys][mic]amix=inputs=2:duration=longest:normalize=0[a]")),
        (Some(c), None) | (None, Some(c)) => Some(format!("{c}[a]")),
        (None, None) => None,
    };

    let graph = match &audio {
        Some(au) => format!("{video};{au}"),
        None => video,
    };
    a.extend(["-filter_complex".into(), graph, "-map".into(), "[v]".into()]);
    a.extend(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "23"].map(String::from));
    if audio.is_some() {
        a.extend(["-map", "[a]", "-c:a", "aac", "-b:a", "160k", "-ac", "2"].map(String::from));
    }
    a.push(p.output_path.clone());
    a
}

// One recording attempt with a given backend, including its system-sound feed.
async fn attempt(b: Backend, p: &RecordParams, area: &Rect) -> Result<(Child, Option<loopback::Loopback>), String> {
    let source = if p.system_audio { Some(loopback::Source::open()?) } else { None };
    let args = record_args(b, p, area, source.as_ref());
    // Start feeding before ffmpeg opens the socket, so it never waits on us.
    let feed = source.map(loopback::Source::start);
    match spawn_recorder(&args).await {
        Ok(child) => Ok((child, feed)),
        Err(e) => {
            if let Some(f) = feed {
                f.stop();
            }
            Err(e)
        }
    }
}

// Start ffmpeg; Err if it dies during startup (bad device, capture API failure...).
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
    sleep(Duration::from_millis(900)).await;
    if let Ok(Some(_)) = child.try_wait() {
        return Err(format!("ffmpeg failed to start:\n{}", log_tail()));
    }
    Ok(child)
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
    if let Some(dir) = PathBuf::from(&params.output_path).parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {}", dir.display(), e))?;
    }

    // Windows that span two screens get clipped to the screen holding most of them.
    let outputs = win::dxgi_outputs();
    if params.window_hwnd.is_some()
        && pick_backend(&area, &outputs) == Backend::Gdi
        && let Some(o) = outputs.iter().max_by_key(|o| overlap(&area, o))
    {
        area = clip(&area, o);
    }

    let backend = pick_backend(&area, &outputs);
    let ((child, feed), used) = match attempt(backend, &params, &area).await {
        Ok(c) => (c, backend),
        // Desktop Duplication can be unavailable (remote desktop, old drivers): fall back to GDI.
        Err(_) if backend != Backend::Gdi => (attempt(Backend::Gdi, &params, &area).await?, Backend::Gdi),
        Err(e) => return Err(e),
    };

    *state.active.lock().unwrap() = Some((child, PathBuf::from(&params.output_path), feed));
    // The floating controls and border are hidden from capture, so they never show up in the video.
    let _ = open_recording_chrome(&app, &area, params.show_border);
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

fn open_recording_chrome(app: &tauri::AppHandle, area: &Rect, border: bool) -> tauri::Result<()> {
    close_recording_chrome(app);

    if border {
        const B: i32 = 3;
        let w = helper_window(app, "border", serde_json::json!({ "view": "border" }))
            .transparent(true)
            .focused(false)
            .build()?;
        exclude(&w);
        w.set_ignore_cursor_events(true)?;
        w.set_position(tauri::PhysicalPosition::new(area.x - B, area.y - B))?;
        w.set_size(tauri::PhysicalSize::new(area.width + 2 * B as u32, area.height + 2 * B as u32))?;
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
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let bar = helper_window(app, "recbar", serde_json::json!({ "view": "recbar", "startedAt": started, "area": area }))
        .inner_size(440.0, 56.0)
        .transparent(true)
        .build()?;
    exclude(&bar);
    if let Some(m) = m {
        let scale = m.scale_factor();
        let w = (440.0 * scale) as i32;
        bar.set_position(tauri::PhysicalPosition::new(
            m.position().x + (m.size().width as i32 - w) / 2,
            m.position().y + (16.0 * scale) as i32,
        ))?;
    }
    bar.show()?;
    Ok(())
}

fn close_recording_chrome(app: &tauri::AppHandle) {
    for label in ["recbar", "border"] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.destroy();
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
    let (child, out, feed) = state.active.lock().unwrap().take().ok_or("no recording in progress")?;
    tauri::async_runtime::spawn_blocking(move || {
        let r = finish(child);
        if let Some(f) = feed {
            f.stop();
        }
        r
    })
    .await
    .map_err(|e| e.to_string())??;

    if !out.exists() {
        return Err(format!("recording file was not created:\n{}", log_tail()));
    }
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

// Names of DirectShow audio capture devices (mics, "Stereo Mix", etc.).
#[tauri::command]
async fn list_audio_devices() -> Result<Vec<String>, String> {
    let output = tauri::async_runtime::spawn_blocking(|| {
        ffmpeg()
            .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
            .stdin(Stdio::null())
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(spawn_err)?;
    Ok(parse_dshow_audio(&String::from_utf8_lossy(&output.stderr)))
}

fn parse_dshow_audio(text: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    text.lines()
        .filter(|l| l.trim_end().ends_with("(audio)"))
        .filter_map(|l| {
            let start = l.find('"')? + 1;
            let end = l.rfind('"')?;
            (end > start).then(|| l[start..end].to_string())
        })
        // The same device can be listed twice.
        .filter(|d| seen.insert(d.clone()))
        .collect()
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
    loopback::output_name()
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

// Open a transparent drag-to-select overlay on every monitor. The overlay
// calls `finish_selection`, which closes them all and sends the area to main.
#[tauri::command]
async fn open_selector(app: tauri::AppHandle, aspect: Option<f64>, current: Option<Rect>) -> Result<(), String> {
    close_overlays(&app);
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let mut overlays = Vec::new();
    for (i, m) in monitors.iter().enumerate() {
        let cfg = serde_json::json!({
            "view": "overlay",
            "originX": m.position().x,
            "originY": m.position().y,
            "aspect": aspect,
            "current": current,
        });
        let w = helper_window(&app, &format!("overlay-{stamp}-{i}"), cfg)
            .transparent(true)
            .build()
            .map_err(|e| e.to_string())?;
        // Physical units so mixed-DPI monitors line up exactly.
        w.set_position(*m.position()).map_err(|e| e.to_string())?;
        w.set_size(*m.size()).map_err(|e| e.to_string())?;
        w.show().map_err(|e| e.to_string())?;
        overlays.push(w);
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

fn close_overlays(app: &tauri::AppHandle) {
    for (label, w) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = w.destroy();
        }
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
    if let Some(main) = app.get_webview_window("main") {
        // When recording straight away the main window stays out of the way.
        if recording {
            let _ = main.show();
            let _ = main.minimize();
        } else {
            let _ = main.show();
            let _ = main.unminimize();
            let _ = main.set_focus();
        }
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
                && let Some((child, _, feed)) = app.state::<Recorder>().active.lock().unwrap().take()
            {
                let _ = finish(child);
                if let Some(f) = feed {
                    f.stop();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dshow_audio_devices() {
        let text = r#"[in#0 @ 0000] "Integrated Camera" (video)
[in#0 @ 0000]   Alternative name "@device_pnp_x"
[in#0 @ 0000] "Microphone Array (Realtek(R) Audio)" (audio)
[in#0 @ 0000]   Alternative name "@device_cm_y"
[in#0 @ 0000] "Stereo Mix (Realtek(R) Audio)" (audio)
[in#0 @ 0000] "Microphone Array (Realtek(R) Audio)" (audio)
dummy: Immediate exit requested"#;
        assert_eq!(
            parse_dshow_audio(text),
            vec!["Microphone Array (Realtek(R) Audio)", "Stereo Mix (Realtek(R) Audio)"]
        );
    }

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
mod loopback_probe {
    // cargo test loopback_to_file -- --ignored --nocapture   (play some sound meanwhile)
    #[test]
    #[ignore]
    fn loopback_to_file() {
        let out = std::env::temp_dir().join("qz-loopback-test.m4a");
        let src = super::loopback::Source::open().unwrap();
        let mut args = vec!["-y".to_string()];
        args.extend(src.ffmpeg_input());
        args.extend(["-t", "4", "-c:a", "aac"].map(String::from));
        args.push(out.to_string_lossy().into_owned());
        let feed = src.start();
        let status = super::ffmpeg().args(&args).status().unwrap();
        feed.stop();
        assert!(status.success());
        println!("wrote {}", out.display());
    }
}

#[cfg(test)]
mod sync_probe {
    use super::*;
    // cargo test sync_probe -- --ignored --nocapture   (with the sync clip playing at 1920,159 640x360)
    #[test]
    #[ignore]
    fn sync_probe() {
        let out = std::env::temp_dir().join("qz-sync-rec.mp4");
        let n: Vec<i32> = std::env::var("QZ_AREA").unwrap_or("1920,159,640,360".into()).split(',').map(|v| v.parse().unwrap()).collect();
        let area = Rect { x: n[0], y: n[1], width: n[2] as u32, height: n[3] as u32 };
        let p = RecordParams {
            output_path: out.to_string_lossy().into_owned(),
            fps: 30,
            area,
            window_hwnd: None,
            audio_device: std::env::var("QZ_MIC").ok(),
            system_audio: true,
            output_short_side: None,
            draw_mouse: false,
            show_border: false,
        };
        let backend = pick_backend(&area, &win::dxgi_outputs());
        let (child, feed) = tauri::async_runtime::block_on(attempt(backend, &p, &area)).unwrap();
        std::thread::sleep(Duration::from_secs(std::env::var("QZ_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(11)));
        finish(child).unwrap();
        if let Some(f) = feed {
            f.stop();
        }
        println!("backend {backend:?} -> {}", out.display());
    }
}
