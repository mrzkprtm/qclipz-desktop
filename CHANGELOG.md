# Changelog

## 0.9.1

### Fixed
- **Low frame rate with audio on.** Recording with system sound or a microphone could drop to 3–8 fps (and slow other apps), because ffmpeg paced the screen grab to the live audio inputs. Audio is now captured by QClipz itself and muxed in when you stop. Video stays at full speed (about 28–29 fps at 1366×768 on a 2011 i5 laptop).
- **Audio/video sync.** Every track shares one clock and is aligned exactly when saving; measured within about 50 ms.
- **Quiet audio.** New **Boost quiet audio** option (on by default) raises quiet recordings to a normal loudness (about −16 LUFS) with a limiter. It never makes audio quieter.

### Changed
- Microphones are listed and recorded through WASAPI (no DirectShow), with the Windows default first and lower latency.
- The recorder runs below normal priority, so the game or video you're recording stays smooth.
- Stopping takes a moment longer when audio is on (the final mux).

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
