# Changelog

## 0.9.2

### Fixed
- **Stutter and screen "blink" right after pressing Record.** The floating bar and border used to be created at that moment (each one starts a browser renderer), stealing focus and CPU for about a second. They're now created once when QClipz starts and only shown when recording begins, without taking focus.
- **The border no longer slows down what you record.** It's an opaque ring-shaped window instead of a transparent window covering the whole area.
- **The main window hides instantly** instead of playing the minimize animation.
- **A saved area that no longer fits** (for example after unplugging a monitor) is moved back onto a screen. **Refresh preview** also picks up monitors you plug in or out.

### Changed
- About 25% less CPU while recording: lighter colour conversion and a low-latency encoder setting. This leaves more headroom for games and video.
- The installer lets you choose **just for me** or **for everyone** and the install folder, and shows the license.
- New app icon, the **Clip** mark.
- The repository has a full README, contributing guide, security policy, and issue and pull-request templates.

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
