# Contributing to QClipz

Thanks for helping! Bug reports, ideas and pull requests are all welcome.

- **Found a bug?** Open a [bug report](https://github.com/mrzkprtm/qclipz-desktop/issues/new?template=bug_report.yml) and attach `%TEMP%\qclipz-ffmpeg.log`.
- **Have an idea?** Start a [discussion](https://github.com/mrzkprtm/qclipz-desktop/discussions) or open a [feature request](https://github.com/mrzkprtm/qclipz-desktop/issues/new?template=feature_request.yml).
- **Security issue?** Report it privately (see [SECURITY.md](SECURITY.md)).

## Development setup

You need Node 20+, Rust (stable, MSVC) and the [Tauri prerequisites for Windows](https://tauri.app/start/prerequisites/). QClipz is Windows-only (Win32, DXGI, WASAPI).

```bash
npm install
npm run dev:app
```

`dev:app` uses its own app ID (`com.qclipz.desktop.dev`), so it runs next to an installed QClipz and keeps separate settings.

FFmpeg is a sidecar. For local recording either have `ffmpeg` on `PATH`, or copy a 64-bit build with `ddagrab`, libx264 and dshow to `src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe`. That file is git-ignored; CI uses an empty placeholder.

## Project layout

| Path | What's there |
|---|---|
| `src/App.tsx`, `App.css` | Main window: source tabs, screen map, output rail, ON AIR, review/crop |
| `src/Selector.tsx` | The drag-to-select overlay shown on every monitor |
| `src/Chrome.tsx` | Floating ON AIR bar and recording border windows |
| `src/theme.css` | Design tokens and shared controls (see `DESIGN.md`) |
| `src-tauri/src/lib.rs` | Commands, the recording session (`begin` / `end`), mux, helper windows |
| `src-tauri/src/audio.rs` | WASAPI capture (system loopback + mic) to timestamped tracks |
| `src-tauri/src/win.rs` | Small hand-written Win32/DXGI bindings |
| `src-tauri/tauri.conf.json` | App, CSP, bundle and installer config |

## Checks

```bash
npm run build                       # type-check + frontend build
cd src-tauri && cargo clippy && cargo test
```

To measure frame rate and audio/video sync through the real pipeline (no UI), use the ignored probe test. Its doc comment in `lib.rs` lists the environment variables:

```bash
cd src-tauri
QZ_AREA=0,0,1366,768 QZ_SECS=10 cargo test sync_probe -- --ignored --nocapture
```

## Pull requests

- Keep changes focused; one topic per PR.
- Match the surrounding code style. Comments explain *why*, not what.
- Update `CHANGELOG.md` for anything a user would notice.
- Don't commit recordings, logs or screenshots of your own desktop.

By contributing you agree that your work is licensed under the [MIT License](LICENSE).
