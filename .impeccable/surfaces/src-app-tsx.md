---
version: 1
slug: "src-app-tsx"
primary_target: "src/App.tsx"
related_targets: ["src/Selector.tsx"]
---

Scope: QClipz desktop app — main window (setup, recording, review), drag-select overlay, floating recording bar, border. Mode: Operate.
Audience/job: one person recording their Windows screen (shorts, tutorials, gameplay, work); pick source → record → crop/save in seconds.

## Direction contract
THESIS: The recorder is a studio camera's tally system. Armed is tally green, live is tally red, and the Record control is a real ON AIR lightbox. Refuses the category default: grey panels with a lone red dot.
OWN-WORLD: Studio-black ground tinted toward camera-body warm graphite. A strict 9-step neutral ramp. Tally green #2BD46A means ready/selected, tally red #FF2D3D means live/record, amber means warnings. Headings and the lightbox use condensed sign lettering (Barlow Condensed); UI text uses Barlow; timecode uses tabular figures. Rules are 1px, radii are 10px, and controls are standard tabs, segmented controls, selects and toggles.
STORY: The user sees what will be captured (a monitor map at exact scale with a live frame), picks a size and fps, and lights ON AIR. While recording, a floating tally bar shows a timecode and Stop.
FIRST VIEWPORT: The left work area holds the source tabs (Screen | Area | Window) over a large preview monitor. The monitor shows the desktop snapshot at true relative scale, with the chosen area framed in a tally-green border and its dimensions. The right rail holds output settings, and the ON AIR lightbox spans the rail's foot as the primary action.
FORM: Tally light / ON AIR sign, position 3 of 7 on the ordered list, seed 9be2000d. Raises: hard relay switches (steps, no fades) except the lamp warm-up; exact-scale monitor map; strict tonal ramp; dense whole-cell window grid; red only where something can be acted on.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
