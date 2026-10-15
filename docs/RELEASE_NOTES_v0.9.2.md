QClipz 0.9.2 makes recording start smoothly and run lighter, and adds a new icon. Recommended for everyone on 0.9.x.

**Download:** `QClipz_0.9.2_x64-setup.exe` below. It updates older versions in place; you can now choose *just for me* or *for everyone* and the install folder.

> The installer isn't code-signed yet. If Windows shows "Windows protected your PC", click **More info → Run anyway**. SHA-256: `<fill in>`

### Fixed
- **Smooth start.** No more stutter or screen blink right after Record. The floating bar and border are prepared when QClipz starts and appear without taking focus.
- **The border no longer costs smoothness.** It's an opaque ring-shaped window instead of a transparent window over the whole area.
- **No minimize animation.** The main window hides instantly while you record.
- **Monitor changes.** A saved area that no longer fits a screen is pulled back on screen, and Refresh preview re-reads your monitors.

### Changed
- About 25% less CPU while recording, which leaves more headroom for the game or video you're capturing.
- Installer: choose per-user or all-users and the install location.
- New **Clip** app icon.

### Licenses
QClipz is MIT-licensed. This installer bundles FFmpeg 8.1.1 (gyan.dev full build, GPLv3); its source is attached as `ffmpeg-8.1.1.tar.xz`. See THIRD_PARTY_NOTICES.md.
