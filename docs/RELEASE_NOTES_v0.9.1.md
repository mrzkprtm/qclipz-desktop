QClipz 0.9.1 fixes recording performance and audio. Update recommended for everyone on 0.9.0.

**Download:** `QClipz_0.9.1_x64-setup.exe` below. It updates 0.9.0 in place and installs per user with no admin rights.

> The installer isn't code-signed yet. If Windows shows "Windows protected your PC", click **More info → Run anyway**. SHA-256: `507a7d4ddc383002f8a9b3b5282f6753c398e7c5dc83bc9baa382d893a6be991`

### Fixed
- **Low frame rate with audio on.** With system sound or a mic, capture could drop to 3–8 fps and slow other apps (for example, YouTube buffering). Video now records at full speed while audio is captured separately and muxed when you stop.
- **Audio sync.** Audio and video share one clock and are aligned exactly on save (measured within about 50 ms).
- **Quiet audio.** New **Boost quiet audio** option (on by default) brings low-volume recordings up to a normal level.

### Changed
- Microphones are captured through WASAPI with lower latency; the Windows default device is listed first.
- The recorder runs at below-normal priority, so what you're recording stays smooth.

### Licenses
QClipz is MIT-licensed. The installer bundles FFmpeg 8.1.1 (gyan.dev full build, GPLv3); its source is attached to this release as `ffmpeg-8.1.1.tar.xz`. See THIRD_PARTY_NOTICES.md.
