# Issue #82 repair and desktop retest

Source: [issue #82 and its numbered follow-ups](https://github.com/yuxino/kiri/issues/82).
The report used v1.6.7 on Debian 13 / XFCE / X11, WebKitGTK 2.54 and
GStreamer 1.26. Its evidence establishes that environment; it does not establish
the same failures on every supported platform.

## Scope and evidence

Code inspection supports the shared editor, metadata and keyboard defects.
Regression tests cover those paths with injected data. The HTTP playback route
addresses WebKitGTK's rejection of custom media schemes without changing its
protocol security settings. Countdown focus and recording startup are desktop
timing changes: their effectiveness requires native retesting.

The original report correctly excluded Linux audio, click ripples, video editing,
automatic updates and GIF import, plus Return committing screenshot text. Those
are existing product boundaries. A single unrepeated black GIF opening was also
not established as a defect. No change was made for these observations.

## Retest matrix

Use the candidate's actual commit and package checksum. Preserve inputs and keep
test captures separate from personal captures. For every result, record the
desktop/media versions, exact steps, expected and observed behavior, repetition
count and evidence. Mark blocked or partial checks explicitly.

| Item | Repair | Desktop acceptance |
| --- | --- | --- |
| 1 | Recalculate text layout and hit bounds while changing size, including old incorrect bounds. | Enlarge wrapped multiline text; select its last line, save/reopen, resize and undo. |
| 2 | Let the editor's measured canvas shrink with the flex viewport. | Shrink square-image and QR editors; inspect the bottom edge and details popup. |
| 3 | Hide the selected screenshot hint when annotation controls appear. | Select a full display and inspect all toolbar icons. |
| 4 | Successful empty OCR returns an empty result instead of an engine error. | Recognize a blank image; expect an empty state, no Retry and no saved text record. |
| 5 | Linux video uses a bounded, process-scoped loopback media capability. | Play imported MP4/MOV and a Kiri recording in the library, viewer and toast; pause, seek and continue playback. Repeat on WebKitGTK 2.54. |
| 6 | Map/present/focus the Linux countdown synchronously before restoring the source. | From another application, press Esc while 3 or 2 is visible, without clicking; no recording or asset may follow. |
| 7 | Stationary resize/endpoint clicks skip geometry and history changes. | Move a mark, click a handle without dragging, undo once; the move must be undone. |
| 8 | Reused editor/pin windows unminimize; the pin action restores always-on-top. | Keep a draft, minimize and reopen from the library. Unpin/minimize a pin, then pin it again; check native stacking and button state. |
| 9 | PNG exports retain source alpha. | Save As without changes and Save with a small mark; inspect unchanged transparent and semitransparent pixels. |
| 10 | Keyboard font changes update the selected mark and group undo on keyup/blur. | Change size with arrows, Home/End and Page keys; check content, bounds, persistence and one-step undo. |
| 11 | Linux decoding applies rotation metadata before scale and GIF conversion. | Convert a quarter-turn MP4; inspect orientation, dimensions, thumbnail and frames. |
| 12 | OCR result text permits WebKit text selection; its shortcuts remain local. | Drag-select a substring, copy and paste it; keep the result popup open. |
| 13 | Merge only changed preference fields under the native lock; serialize pending saves. | Change different fields in two editors/overlay windows and close promptly; reopen and inspect both fields. |
| 14 | Crop the clean source/document before rendering; include text-background padding. | Crop through a text background and mosaic stroke; compare first save against reopened, unedited export pixel for pixel. |
| 15 | Drain warmup samples/old PTS and establish output time at the first accepted Linux frame. | Repeat MP4 and direct GIF starts, with panels inside the region; extract the first several frames and inspect handles/countdown/controls. Repeat after pause/resume. |
| 16 | Normalize tag identity; preserve a visible cancel chip; clear vanished unsearched tags. | Use QA/qa, toggle either spelling, search to zero results, delete/untag the last matching asset. |
| 17 | Text input and Canvas use explicit eight-space tab stops. | Enter tabbed text; compare input, committed canvas, saved PNG and reopen. |
| 18 | Input and Canvas share content insets with native editing slack. | Type through the final glyph of a single-line box; compare live input and committed output. |
| 19 | Restore the Linux background library without queued activation. | Keep the library visible behind another app; complete a capture and inspect immediate and delayed focus. |
| 20 | Give focused controls their native Enter action before the capture shortcut. | Tab to Pen/Rectangle and press Enter; select the tool without completing capture. Confirm ordinary Return completion and OCR consent's local Return behavior. |
| 21 | OCR correction emits the asset update event and refreshes open dialogs. | Keep a source viewer and Read Text open through two History corrections; inspect and copy the newest text. |
| 22 | Convert supported ICC images to sRGB and tag the output, preserving alpha/precision. | Import AdobeRGB PNG/JPEG and an sRGB reference; compare pixels/colors, reopen and crop. Hash the untouched input files. |
| 23 | Repeat local dense-QR scans at the same contrast thresholds as whole-image scans. | Scan the mixed-contrast 12-code grid twice; verify all content/positions, plus clear 9/16-code controls and the isolated low-contrast code. |

## Verification boundaries

Run the repository's required Rust, frontend and diff checks, then the signed
fixed-path macOS app and the CI-built/installed Ubuntu `.deb`. The isolated X11
suite includes actual video progress, pause, seek followed by playback, countdown
Esc and source-focus checks. Separate GNOME Wayland runs cover portal behavior
and display scales. These do not replace the original XFCE/WebKitGTK retest,
real GPU, mixed-monitor or IME acceptance.

The startup gate uses at least 150 ms of source time and three frames. This is a
bounded mitigation, not proof that every compositor has removed every overlay
by that instant. Keep #82 open until native evidence establishes the result.
