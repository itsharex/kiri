# Kiri integrated acceptance candidate

This draft candidate combines independently reviewed fixes on main `1851f58`.
It does not merge or release the fixes, close reporter issues, or implement the
QR (#55) and Wayland GlobalShortcuts Portal (#47) feature requests.

| Source issue / PR | Included change | Existing evidence | Integrated acceptance still needed |
| --- | --- | --- | --- |
| #63 / #64 (`9d177f8`, `4cbc9dd`) | Reject stale macOS frozen/current display geometry before starting or resuming recording; explain recovery | Source CI and injected geometry boundaries | Signed Mac native; physical display changes / mixed DPI |
| #65 / #66 (`18259a1`, `d6cf7e4`) | Compact shared annotation toolbar, measured viewport placement and expandable parameters | Independent exact-package X11 virtual 800×600 native mouse Done → 140×160 PNG | Same scenario on the integrated package; other native platforms |
| #54 / #68 (`0bf8303`) | Text undo, two-stage Escape, newline hint and unsaved image close guard | Source CI, 18 actual built-browser component paths | Native text / editor / real IME on the integrated package |
| #67 / #69 (`ea55c7a`) | Cumulative GIF centisecond timing and actual encoded metadata duration | Real Rust GIF fixture full decode and platform media tests | Same original 154.156667 s MP4 converted twice in native GUI |
| #48 / merged #50 | Existing error-toast icon fallback retained from main | Previous Ubuntu24/GNOME46 native error path and current renderer tests | Integrated error-path replay; not a claim of capture success |
| #21 | Preserve platform multi-display and configurable shortcut tracking | Current main and fixes above | Physical mixed DPI, hotplug/default display, conflict recovery |

The OCR, toolbar and image-editing renderer checks share the existing CI entry
point and run sequentially, with each browser fixture closed before the next.
No synthetic capture mode is added to the application. Existing source-PR
screenshots remain labelled by their own version and environment.

## One exact-package Linux replay

The PR body records final head, actual CI checkout (PR merge candidate), artifact
identity and Debian SHA256. Read `provenance.json` and verify its package digest
before installing the candidate in the isolated QA profile. Do not reuse source
PR acceptance as proof of the integrated package.

1. On the same virtual X11 outputs (1364×1024 primary plus 800×600 secondary,
   scale 1), select local (650,420)→(790,580). Check More/Text/Mosaic placement,
   focused More Enter/Space, dimension-field Enter, and mouse Done's 140×160 PNG.
2. Check screenshot pixels, OCR copy, PNG Save As, text Undo/Redo, Shift+Enter,
   first/second Escape and real IME. Exercise image Save/Discard/Keep editing,
   cancelled Save As and failure retention. Record fixture-only failures separately.
3. Record only public patterns. Check countdown cancellation, start/stop,
   pause/resume, complete MP4 decode and absence of Kiri controls in frames.
4. Reuse the original public 154.156667 s, 560×300 source MP4. Convert twice,
   decode every GIF frame, compare delays/total duration, and require stored
   duration to equal actual encoded duration. Keep unedited images and originals.
5. Check shortcut conflict recovery, language/restart persistence and safe error
   notices. Restore the virtual layout and stop all task recording/test windows.
   Only move task-created captures to recoverable Trash; never empty user Trash.

## Evidence boundaries

Debian13/Xfce/X11 is additional compatibility evidence, outside the documented
Ubuntu24/GNOME target. Virtual mixed resolutions at scale 1 are not physical mixed
DPI. Browser fixtures, Xvfb and isolated GNOME Wayland CI have separate labels.
The available Mac inventory is one built-in display; physical hotplug/primary
changes remain untested. Capture freezes one active display, not cross-display
composition; current Wayland capture rejects multiple connected displays.
Linux video editing remains unavailable by contract. A Debian WebKit preview
failure has not been established as a product defect. QR and Portal setup remain
unimplemented. Keep all acceptance gaps visible in the unified PR body.

A failed-jobs-only GitHub rerun currently increments run_attempt while retaining
the old package manifest. Strict existing provenance rejects that mismatch. Do
not relabel the package or bypass the check; record the CI infrastructure limit
and use a completed matching candidate. No harness relaxation is included here.
