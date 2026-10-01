# X11 source readiness during recording acceptance

Run [36840768722](https://github.com/yuxino/kiri/actions/runs/36840768722)
built application source `1032a88eae14206aafb4e6bf9790670cf19f8ae6`.
The Debian package artifact is `11151129464`; package SHA-256 is
`0d3c1dedb488541af67d3701d1f46b4f7be2c807e2fb0c85d003e653f389c63a`.
The packaging step succeeded, but X11 acceptance and the quality gate failed.
The same-package GNOME Wayland checks passed at 1/1.25/1.5/2 compositor scales.
Keep the failed run; it is not an accepted candidate.

The actual failed image is uniformly grey, with no selection handles, badge,
or recording controls. Complete offline decoding of the actual 680×380 MP4
found 197 frames: frames 0–2 grey, 3–102 the initial pattern, and 103–196 the
resumed pattern. No paused-pattern frames appeared. This is different from
the confirmed #72 selection-window contamination on the earlier b8 package.

The original driver owned the source's Tk event loop. `subprocess.run` waited
for `xdotool click` (including its mouse-up delay) without pumping Tk. Kiri's
synchronous unmap generated X11 Expose events, but the driver could not repaint
the source until the command returned. The approximately 100 ms grey interval
matches that driver stall. An X-server round trip confirms Kiri's unmap, not
another process's handling of Expose; the test must let that process run.

The public source now runs in its own ordinary managed X11 process with a
continuous Tk main loop. The controller sends show/hide/pattern commands and
waits for acknowledgements. The source records monotonic command/Expose events.
A source-only integration probe covers/unmaps the pattern while the driver is
deliberately blocked; before/after desktop pixels must match before Kiri is
tested. It does not provide pixels to the app or modify its capture backend.

The recording decoder retains the same mean/outlier/edge thresholds and checks
every frame, including frame 0, paused content, dimensions and timestamps.
Failure evidence additionally records the frame index, PTS, matching reference,
and metrics. No frame is discarded, no timestamp/FPS is changed, and no startup
sleep is added to Kiri. Application/build sources remain unchanged.

Remote X11 revalidation is required to establish that the source-loop correction
resolves this failure. Cloud native #72 start/pause/resume/countdown/restart
acceptance remains separate; a harness correction cannot close the product issue.
