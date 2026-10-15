---
name: QClipz
description: A Windows screen recorder dressed as a studio camera's tally system.
colors:
  n0: "#0e0c0d"
  n1: "#151213"
  n2: "#1c1819"
  n3: "#262122"
  n4: "#332c2d"
  n5: "#4a4142"
  n6: "#6f6566"
  n7: "#a69b9c"
  n8: "#d9d0d0"
  n9: "#f6f0ee"
  tally-green: "#2bd46a"
  tally-green-wash: "rgba(43, 212, 106, 0.12)"
  tally-green-ink: "#062611"
  tally-red: "#ff2d3d"
  tally-red-wash: "rgba(255, 45, 61, 0.14)"
  lit: "#c91726"
  lit-letters: "#ffffff"
  glass-off: "#2a0e11"
  glass-off-rim: "#4a1a1f"
  letters-off: "#c4434d"
  letters-off-sub: "#e8838b"
  amber: "#ffb020"
  scrim: "rgba(8, 6, 7, 0.55)"
typography:
  sign:
    fontFamily: "Barlow Condensed, Barlow, Segoe UI, sans-serif"
    fontSize: "40px"
    fontWeight: 700
    lineHeight: 1
    letterSpacing: "0.14em"
  sign-hint:
    fontFamily: "Barlow Condensed, Barlow, Segoe UI, sans-serif"
    fontSize: "30px"
    fontWeight: 600
    letterSpacing: "0.01em"
  brand:
    fontFamily: "Barlow Condensed, Barlow, Segoe UI, sans-serif"
    fontSize: "22px"
    fontWeight: 700
    letterSpacing: "0.02em"
  heading:
    fontFamily: "Barlow Condensed, Barlow, Segoe UI, sans-serif"
    fontSize: "17px"
    fontWeight: 700
    letterSpacing: "0.02em"
  tag:
    fontFamily: "Barlow Condensed, Barlow, Segoe UI, sans-serif"
    fontSize: "15px"
    fontWeight: 700
    letterSpacing: "0.02em"
    fontFeature: "\"tnum\" 1"
  body:
    fontFamily: "Barlow, Segoe UI, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
  control:
    fontFamily: "Barlow, Segoe UI, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 500
    lineHeight: 1.45
  small:
    fontFamily: "Barlow, Segoe UI, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
  key:
    fontFamily: "Barlow, Segoe UI, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 600
rounded:
  hair: "3px"
  chip: "4px"
  tag: "5px"
  sm: "6px"
  md: "8px"
  lg: "10px"
  xl: "12px"
  full: "9999px"
spacing:
  xs: "6px"
  sm: "8px"
  md: "10px"
  lg: "14px"
  xl: "16px"
  gutter: "18px"
components:
  button:
    backgroundColor: "{colors.n2}"
    textColor: "{colors.n9}"
    typography: "{typography.control}"
    rounded: "{rounded.sm}"
    padding: "0 14px"
    height: "34px"
  button-hover:
    backgroundColor: "{colors.n3}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.n8}"
    rounded: "{rounded.sm}"
    height: "34px"
  button-danger-hover:
    textColor: "{colors.tally-red}"
  field:
    backgroundColor: "{colors.n0}"
    textColor: "{colors.n9}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "34px"
  segmented:
    backgroundColor: "{colors.n0}"
    rounded: "{rounded.md}"
    padding: "3px"
  segmented-option:
    backgroundColor: "transparent"
    textColor: "{colors.n7}"
    typography: "{typography.control}"
    rounded: "{rounded.tag}"
    padding: "0 11px"
    height: "28px"
  segmented-option-selected:
    backgroundColor: "{colors.n3}"
    textColor: "{colors.n9}"
  switch-track:
    backgroundColor: "{colors.n4}"
    rounded: "{rounded.full}"
    width: "34px"
    height: "20px"
  switch-track-on:
    backgroundColor: "{colors.tally-green}"
  lamp:
    backgroundColor: "{colors.n5}"
    rounded: "{rounded.full}"
    size: "8px"
  lamp-armed:
    backgroundColor: "{colors.tally-green}"
  lamp-live:
    backgroundColor: "{colors.tally-red}"
  window-cell:
    backgroundColor: "{colors.n1}"
    textColor: "{colors.n9}"
    rounded: "{rounded.md}"
    padding: "10px 12px"
    height: "80px"
  window-cell-selected:
    backgroundColor: "{colors.tally-green-wash}"
  frame-tag:
    backgroundColor: "{colors.tally-green}"
    textColor: "{colors.tally-green-ink}"
    typography: "{typography.tag}"
    rounded: "{rounded.chip}"
    padding: "1px 7px"
  onair:
    backgroundColor: "{colors.glass-off}"
    textColor: "{colors.letters-off}"
    typography: "{typography.sign}"
    rounded: "{rounded.lg}"
    height: "92px"
  onair-lit:
    backgroundColor: "{colors.lit}"
    textColor: "{colors.lit-letters}"
  recbar:
    backgroundColor: "{colors.n1}"
    rounded: "{rounded.xl}"
    padding: "0 6px 0 4px"
  recbar-sign:
    backgroundColor: "{colors.lit}"
    textColor: "{colors.lit-letters}"
    rounded: "{rounded.md}"
    padding: "0 12px"
    height: "38px"
  recbar-stop:
    backgroundColor: "{colors.n3}"
    textColor: "{colors.n9}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "34px"
  floating-toolbar:
    backgroundColor: "{colors.n1}"
    rounded: "{rounded.xl}"
    padding: "6px"
---

# Design System: QClipz

## Overview

**Creative North Star: "The Studio Tally Light"**

QClipz is styled after the tally system on a studio camera. The ground is studio black, tinted warm like a camera body, and built from one graphite ramp. Two lamps carry meaning. Tally green means armed: the area that will be recorded, the selected option, a toggle that is on. Tally red means live: recording is running, or the control in front of you will start or stop it. The primary action is an actual ON AIR lightbox. When idle it is unlit red glass, and when recording starts it strikes on as a flat deep-red sign with white lettering. The system rejects the usual recorder look of grey panels with one small red dot.

Density is that of a working tool. Controls are 34px tall, gaps are tight (6–14px), and the main window has a work area beside a 312px settings rail. Nothing glides. A state change is a relay closing, a hard two-step switch at 90ms, and the only animation is the lamp warm-up flicker when a sign lights. Lettering comes in two voices: Barlow for UI text, and Barlow Condensed for anything that reads like a sign (the brand, section heads, size tags, ON AIR, the timecode). Both are self-hosted through @fontsource.

The monitor map tells the truth. Screens are drawn at their exact relative scale over a real desktop snapshot. The capture area is framed in a tally-colored outline with corner brackets and a pixel-size tag, so what you see selected is what gets recorded.

**Key Characteristics:**
- Warm graphite ground from a single 10-step ramp (n0–n9); depth comes from tonal steps, not shadows.
- Green marks state and never fills an action. Red marks live and record.
- The ON AIR lightbox is the primary control: unlit glass when idle, a lit sign while recording.
- Condensed sign lettering for labels that act as signs. Tabular figures for every number that changes.
- Hard relay switches (90ms, steps(2, end)). The only motion is the lamp warm-up, and it respects reduced motion.
- 1px rules, gently rounded controls (6–10px), corner-bracket frames.

## Colors

The palette is a camera body and two lamps: warm graphite neutrals, a green tally, a red tally with its lit-sign field, and amber for advice.

### Primary
- **Live Tally Red** (tally-red): Marks the live state and the record action. Used for the live monitor border, the live frame outline, the live lamp, the 3px border drawn around the recorded area, the overlay Record button, the Stop glyph, and destructive hover (Delete). Its wash (tally-red-wash) tints the live state.
- **Lit Sign Field** (lit): The flat deep red behind lit lettering: the lit ON AIR lightbox and the floating bar's ON AIR sign. White lettering (lit-letters) sits on this field at 5.8:1.

### Secondary
- **Armed Tally Green** (tally-green): Marks state only: the capture frame and its corner brackets, armed lamps, toggles that are on, the selected window cell's border, the focus ring, text selection, and the saved confirmation. Text on a green fill uses Tally Ink (tally-green-ink). The selected-cell tint is tally-green-wash.

### Tertiary
- **Advice Amber** (amber): Small inline advice and warnings, for example the 60 fps warning, the audio device notes, and the "minimized" marker on a window cell. It is always text, never a fill.

### Neutral
- **Studio Black** (n0): Recessed wells: the preview monitor, fields, segmented-control trays.
- **Camera Body** (n1): Window ground, the work area, floating bars, window cells.
- **Rail Graphite** (n2): The settings rail and default button fill.
- **Raised Graphite** (n3): Hover fills, the selected segment, 1px rules, monitor edges.
- **Edge Graphite** (n4 / n5): Control borders (n4) and hover borders (n5). The off-switch track is n4 and an unlit lamp is n5.
- **Dim Ash** (n6): Disabled text and the drag grip only.
- **Muted Ash** (n7): Secondary text: hints, meta, the summary line, unselected segments.
- **Soft Paper** (n8): Secondary readable text: row labels, window titles, ghost buttons.
- **Paper White** (n9): Primary text and emphasis.
- **Unlit Glass** (glass-off, rim glass-off-rim): The ON AIR lightbox at rest. It uses a faint radial glow in the glass, lettering in letters-off and the sub line in letters-off-sub.
- **Scrim** (scrim): Dims the desktop around a drag-selected area and around a crop rectangle.

### Named Rules
**The Tally Rule.** Green reports state and red reports live. Green never fills a button or other action. If it is green, it is already true.

**The Red Is Reserved Rule.** Tally red appears only for record, live, stop, and destructive hover. If a red element cannot be acted on and does not mean "recording now", it is wrong.

**The Lit Field Rule.** White lettering on red always sits on the lit field (#c91726), never on bright tally red. Bright red is for lamps, outlines and glows.

**The One Ramp Rule.** Every neutral comes from n0–n9. Do not introduce in-between greys.

## Typography

**Display Font:** Barlow Condensed (with Barlow, Segoe UI)
**Body Font:** Barlow (with Segoe UI, system-ui)

**Character:** Barlow Condensed is the stencil lettering on a studio sign: tall, tight and confident. Barlow is its plain, legible workshop counterpart for every label and control.

### Hierarchy
- **Sign** (700, 40px, line-height 1, tracking 0.14em): The ON AIR lettering. While lit it shows the timecode with tracking tightened to 0.06em and a soft red letter glow.
- **Sign hint** (600, 30px): The centered instruction on the drag-select overlay.
- **Brand** (700, 22px, 0.02em): The QClipz wordmark in the top bar. The floating bar's timecode uses the same size at 0.03em.
- **Heading** (700, 17px, 0.02em): Rail section heads (Output, Audio, Save to).
- **Tag** (700, 14–15px, 0.02em, tabular): Size tags on frames, screen tags on the monitor map, and window-cell app names (uppercase, 0.04em).
- **Body / Control** (400 / 500, 14px, line-height 1.45): All UI text. Controls use 500 and emphasis uses 600.
- **Small** (400–600, 12–13px): Hints, meta, advice, the summary line, the ON AIR sub line.
- **Key** (600, 11px): Keyboard hints (kbd) with a 1px currentColor outline.

### Named Rules
**The Sign Lettering Rule.** Condensed lettering is only for things that read as signs: the brand, section heads, tags, ON AIR, and timecode. Running UI text, buttons and fields stay in Barlow.

**The Tabular Rule.** Any number that changes or gets compared (timecode, W × H, X/Y/W/H fields, summary line) uses tabular figures.

## Layout

The main window is a two-column grid. The left column is a fluid work area with 16px top and bottom padding and 20px / 18px side padding. The right column is a fixed 312px rail, which narrows to 280px below 1000px. The work area stacks a top bar (brand, source tabs, end actions), the preview monitor (which takes all remaining height), and a bottom strip for meta and actions. The rail scrolls its settings body and pins the ON AIR lightbox in a ruled footer, under a one-line summary of what will be captured and at what size.

Inside the monitor, the screen map fits by container query (min of width and height × ratio) with 28px padding. Screens are positioned at true relative scale. The window picker is a dense grid of whole cells (auto-fill, minmax 210px, 80px rows, 8px gap). Rhythm uses 6, 8, 10 and 14px gaps, and rows and controls share a 34px height.

**The True Scale Rule.** Monitors are drawn at their exact relative size and position over a real snapshot. The map is a measurement, not an illustration.

## Elevation & Depth

Inside the window the system is flat. Depth comes from tonal steps: wells sit at n0, the ground at n1, the rail at n2, and hover or selected surfaces at n3. Edges are 1px rules in n3–n5. Glow means a light is on: armed lamps glow green, live lamps and the lit sign glow red. The lit ON AIR uses an outer red glow plus a 1px red halo. Only the panels that float over an arbitrary desktop (the selection toolbar, the selection action bar) get a soft drop shadow, so they hold up against any wallpaper.

### Shadow Vocabulary
- **Armed lamp** (`box-shadow: 0 0 8px var(--tally-green)`): An armed lamp. Selected segments use a 6px version.
- **Live lamp** (`box-shadow: 0 0 10px var(--tally-red)`): A live lamp.
- **Lit sign** (`box-shadow: 0 8px 34px rgba(255,45,61,.45), 0 0 0 1px rgba(255,45,61,.35)`): The lit ON AIR lightbox.
- **Small lit sign** (`box-shadow: 0 0 14px rgba(255,45,61,.55)`): The floating bar's ON AIR sign.
- **Floating panel** (`box-shadow: 0 10px 30px rgba(0,0,0,.45)`): Toolbars over the desktop only.
- **Scrim cut-out** (`box-shadow: 0 0 0 100vmax rgba(8,6,7,.55)`): Dims everything outside a selection or crop.

### Named Rules
**The Glow Is A Lamp Rule.** A glow means a light is on. Panels inside the window never glow and never cast shadows.

## Shapes

Corners are gently rounded and step by role: 3px for screen tiles and resize handles, 4–5px for tags and segments, 6px for buttons and fields, 8px for segment trays, window cells and recording-bar controls, 10px for the monitor and the ON AIR lightbox, 12px for floating bars. Lamps and switch tracks are fully round. Rules are 1px everywhere. The signature form is the corner bracket: 3px-thick L-shaped brackets (14px in the main window, 18px on the overlay) at each corner of a capture or crop frame, in the frame's tally color. The app icon uses the same language: four green corner brackets around a red record lamp on a rounded graphite tile.

Icons are one stroke set on a 20px grid with a 1.6 stroke, round caps and joins, drawn in currentColor at 16px in buttons and 12–14px in tight slots.

## Components

### Buttons
Plain and tactile, like panel switches.
- **Shape:** gently rounded (6px), 34px tall, 1px n4 border.
- **Default:** n2 fill, n9 label at weight 500, 14px side padding, 7px icon gap.
- **Hover / Focus:** hard switch to n3 fill and n5 border. Focus shows a 2px tally-green outline with a 2px offset.
- **Ghost:** transparent and borderless with an n8 label. Hover fills n3. Used for top-bar refresh and dismiss.
- **Danger:** looks like the default button until hover, when the label and border turn tally red (Delete).
- **Icon:** a 34px square.

### Segmented controls and tabs
- **Style:** an n0 tray with a 1px n3 border, 8px radius and 3px inset. Options are 28px tall (tabs 30px) with n7 labels.
- **State:** the selected option fills n3 with an inset n4 ring and an n9 label. If the option carries a lamp, the lamp turns armed green. Used for source tabs, aspect ratio, frame rate and the screen list.

### Toggle switches
- **Style:** a 34 × 20 track in n4 with an n8 knob. When on, the track turns tally green and the knob turns tally ink, sliding 14px with a hard step.

### Inputs / Fields
- **Style:** an n0 well, 1px n4 border, 6px radius, 34px tall. Selects use a drawn n7 chevron.
- **Focus:** the border turns tally green. The caret is tally green.

### Window cells
A dense grid of whole-cell buttons: an n1 fill, 1px n3 border, 8px radius and 80px tall. Each shows the app name in uppercase sign lettering with a lamp, and a two-line title below. Hover raises the cell to n2 with an n5 border. Selected gets a green border, a green wash and an armed lamp, which turns to a live lamp while recording.

### Tally frame (signature)
The capture outline: a 2px tally outline with corner brackets and a size tag above the top-left corner (green fill, tally-ink lettering, "1080 × 1080"). It turns red while live. The drag-select overlay uses the same frame with a 1px outline, 18px brackets, white-and-green resize handles and the scrim cut-out.

### ON AIR lightbox (signature)
The primary action, 92px tall across the rail foot. When unlit it is a radial-glass field in glass-off with a glass-off-rim border, ON AIR lettering in letters-off, and a "Record Ctrl R" sub line in letters-off-sub. Hover brightens the letters and rim. When lit, the field turns flat lit red and the lettering turns white with a red letter glow. The lettering becomes the running timecode and the sub line becomes "Stop Ctrl R". The lit state strikes on with the lamp warm-up. Pressing it nudges it down 1px.

### Recording bar (signature)
A small floating bar on n1 with an n4 border and 12px radius, which can be dragged by its grip. It holds a small lit ON AIR sign (lit field, white condensed "ON AIR" at 0.14em plus a 22px timecode, small lit-sign glow, warm-up on appear), the recorded size in n7, a restore icon button, and a Stop button on n3 whose stop glyph is tally red. A 3px tally-red border can outline the recorded region. The app hides all of these windows from capture.

### Motion
**The Relay Rule.** State changes are hard switches: `90ms steps(2, end)` on background, border, color and transform. Never use eased fades or slides. The one exception is the lamp warm-up, a 380ms stepped flicker of brightness when a sign lights. Under prefers-reduced-motion it is removed.

## Do's and Don'ts

### Do:
- **Do** use tally green (#2bd46a) only to report state: frame, lamp, toggle-on, selected cell, focus ring.
- **Do** put white lettering on the lit field (#c91726) whenever red carries text.
- **Do** switch states with `90ms steps(2, end)`. Reserve the 380ms lamp warm-up for a sign lighting, and drop it under reduced motion.
- **Do** set sign-like labels (brand, section heads, tags, ON AIR, timecode) in Barlow Condensed 600–700, and everything else in Barlow.
- **Do** use tabular figures for every timecode, size and coordinate.
- **Do** frame capture and crop areas with the 2px tally outline, 3px corner brackets and a pixel-size tag.
- **Do** build depth from the n0–n9 ramp with 1px rules. Only panels floating over the desktop get the drop shadow.

### Don't:
- **Don't** fill a button or action with tally green.
- **Don't** use tally red for anything that is neither live, record, stop nor a destructive hover.
- **Don't** set white text directly on bright tally red (#ff2d3d). It reads at about 3.7:1.
- **Don't** add eased fades, slides or other animation beyond the lamp warm-up.
- **Don't** introduce greys outside the n0–n9 ramp or a second accent hue.
- **Don't** fall back to the category default of grey panels with a lone red dot. Record is the ON AIR sign.
