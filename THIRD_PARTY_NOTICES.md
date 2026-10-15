# Third-party notices

QClipz's own source code is MIT-licensed (see `LICENSE`). The installer also ships
the following third-party software. Their license texts are installed next to the
app in the `licenses` folder (source: `src-tauri/licenses/`).

## FFmpeg (bundled program, GPLv3)

- What: `ffmpeg.exe`, run as a separate program for capture and encoding. QClipz does not link to it.
- Build: `8.1.1-full_build-www.gyan.dev`, the 64-bit static Windows build from https://www.gyan.dev/ffmpeg/builds/
- License: GNU General Public License v3. Full text in `licenses/FFmpeg-GPLv3.txt`.
- Corresponding source: FFmpeg at commit `239f2c733d`
  (https://github.com/FFmpeg/FFmpeg/commit/239f2c733d). The full build configuration and the
  list of bundled external libraries are in `licenses/FFmpeg-build-info.txt`.
- Each GitHub release of QClipz also attaches the FFmpeg source archive for the bundled
  version. If it's missing from a release, open an issue to request the source; it will be
  provided for at least three years from the release date, as GPLv3 section 6 requires.

## Fonts

- Barlow and Barlow Condensed, by The Barlow Project Authors (https://github.com/jpt/barlow).
  SIL Open Font License 1.1. Texts in `licenses/Barlow-OFL.txt` and `licenses/Barlow-Condensed-OFL.txt`.
  Packaged through Fontsource (`@fontsource/barlow`, `@fontsource/barlow-condensed`).

## Libraries compiled into QClipz

| Component | License |
|---|---|
| Tauri, tauri-plugin-dialog, tauri-plugin-opener, tauri-plugin-single-instance | MIT or Apache-2.0 |
| cpal (audio capture) | Apache-2.0 |
| serde, serde_json | MIT or Apache-2.0 |
| React, React DOM | MIT |
| @tauri-apps/api and plugin bindings | MIT or Apache-2.0 |

Transitive dependencies (audited for v0.9.0) are under permissive licenses (MIT, Apache-2.0,
BSD-3-Clause, Zlib, Unlicense, Unicode-3.0, CC0), except five Rust crates under **MPL-2.0**,
pulled in by Tauri: cssparser, cssparser-macros, dtoa-short, option-ext and selectors. They are used
unmodified, and their source is on crates.io. No GPL code is compiled into QClipz itself.
Run `cargo tree` and `npm ls` for the full list.
