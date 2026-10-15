# Changelog

## 0.9.0 (first public pre-release)

### Recording
- Full screen (any monitor or all), dragged **Area**, or a single **Window**.
- Area overlay on every monitor, with aspect lock, exact-size presets and Whole screen. Record straight from the overlay (`R`).
- Desktop Duplication capture (`ddagrab`): tear-free, and it captures GPU-rendered content (browsers, video, games) instead of black. GDI fallback for areas that span monitors.
- **System sound** via WASAPI loopback, no Stereo Mix needed, optionally mixed with a **microphone**.
- Audio and video share one clock, so they stay in sync.
- Output scaling (2160p–480p by the shorter side), 24/30/60 fps, cursor on/off.

### App
- QClipz windows, the floating stop bar and the recording border are excluded from capture.
- Recordings save automatically to `Videos\QClipz`. Review afterwards to crop, open, show in folder, or delete (to the Recycle Bin).
- Single instance; settings are remembered.
- Per-user NSIS installer with FFmpeg bundled; no admin rights needed.
