# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Tauri 2 desktop app on Windows; the UI is a WebView2 web surface.)

## Users

One person recording their own Windows screen, switching between four jobs: short-form vertical clips (TikTok / Reels / Shorts, 9:16), tutorials and app demos (often with a mic), gameplay and other fast-moving video where smoothness matters, and work captures (bug reports, meetings, quick shares). Sessions are frequent and short: pick what to record, record, trim/crop, save.

## Product Purpose

QClipz is a lightweight Windows screen recorder. Success: the user goes from "I want to record that" to a clean mp4 in a few seconds, with exactly the screen, area, or window they meant, at the size and aspect they need.

## Positioning

Area-first recording: drag a rectangle on any monitor (with aspect lock and size presets) and start recording straight from that overlay. Recording controls and borders stay off the video because QClipz hides its own windows from capture.

## Operating Context

- Multi-monitor desktops with mixed resolutions (the author's setup is 1920×1080 + 1366×768).
- Runs on modest hardware (a 2011 i5 laptop with Intel HD 3000), so capture defaults to 30 fps and the encoder is tuned for speed.
- Output goes to a folder automatically (default Videos\QClipz), with an optional crop afterwards.

## Capabilities and Constraints

- Sources: full screen (one monitor or all), dragged area, single window.
- Output: original size or scaled by shorter side (2160/1440/1080/720/480), 24/30/60 fps, cursor on/off. Audio: system sound (WASAPI loopback, no Stereo Mix) and/or a DirectShow microphone, mixed and kept on one clock with the video.
- Capture uses ffmpeg (Desktop Duplication `ddagrab`; GDI fallback for areas that span monitors). ffmpeg ships inside the installer.
- Windows 10 2004+ (exclude-from-capture). No pause/resume yet, no global hotkey yet.
- License: MIT for QClipz; the installer bundles FFmpeg (GPLv3) and Barlow fonts (OFL).

## Brand Commitments

- Name: QClipz. UI language: English.

## Evidence on Hand

No testimonials, users, benchmarks, or marketing claims exist; none may be invented.

## Product Principles

1. Fewest steps from intent to recording; settings remember themselves.
2. What you see selected is exactly what gets recorded, pixel for pixel.
3. QClipz never appears in its own recordings.
4. Honest about the machine: show what will be captured and at what size before recording starts.
