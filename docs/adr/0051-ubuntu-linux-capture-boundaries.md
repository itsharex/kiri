# ADR 0051: Ubuntu Linux capture boundaries

## Status

Accepted for the experimental Linux implementation. Desktop and package
acceptance are tracked separately in [the Linux guide](../linux.md).

Supersedes the affected decisions in [ADR 0049](0049-linux-screenshot-mvp.md)
and [ADR 0050](0050-linux-wayland-grim-and-hyprland-shortcut.md). Those records
and the contributor's [original PR #20](https://github.com/yuxino/kiri/pull/20)
remain the history of the initial port.

## Context

The initial port focused on Hyprland. The first maintained Linux target is
Ubuntu 24.04 with GNOME, including X11 compatibility. GNOME Wayland does not
grant an application unrestricted desktop shortcuts, window enumeration, or
capture exclusions. Portal screenshots and ScreenCast permission are separate;
a second selection must not silently map a crop to another display.

Automatically installing a compositor binding changes the user's desktop
configuration and cannot honor the shared configurable-shortcut contract.
Floating controls also risk appearing in a recording because the portal cannot
portably exclude Kiri's windows.

## Decision

1. Use X11 `xcap` for frozen stills and window bounds. X11 retains configurable
   native shortcuts. On Wayland, expose `kiri --capture` for a user-created
   desktop shortcut and retain the app's Capture button. Do not install
   `hyprctl` bindings or FIFO listeners.
2. Limit the initial Wayland path to one connected display, checked before
   starting capture. Use the GNOME Screenshot portal; keep system `grim` as
   an optional path on compatible wlroots compositors. Preserve explicit
   cancellation and report errors rather than selecting another source.
3. Start silent recording through ScreenCast and PipeWire with in-process
   system GStreamer plugins. Require the same display as the frozen still and
   validate the stream dimensions before applying the selected region. Use
   timestamp-aware MP4 encoding and the existing local GIF workflow. System
   audio, microphone, and click highlights remain unavailable.
4. Do not show a floating Linux recording panel. Provide tray pause/resume and
   stop actions plus `kiri --toggle-recording-pause` and
   `kiri --stop-recording` for desktop shortcuts. The countdown ends before
   frames enter the recording. GNOME sessions without visible tray support
   must have another stop route configured before recording.
5. Keep clipboard ownership in GTK so the clipboard remains available after
   the overlay closes on GNOME Wayland. Run local OCR with system Tesseract
   and installed `eng`, `chi_sim`, and `jpn` data. Remote OCR remains an
   explicit opt-in path with Secret Service credentials.
6. Produce an experimental Ubuntu `.deb` with declared runtime dependencies.
   Linux updates are manually downloaded replacement packages; disable Linux
   updater artifacts and in-app installation. AppImage and signed Linux
   updater delivery require separate implementation and acceptance.
7. Keep screenshots editable and the local library shared. Linux videos
   support playback and GIF conversion; do not expose the macOS/Windows video
   editing and export surface without a Linux renderer.

## Consequences

The first release has explicit desktop and media limits instead of implying
macOS/Windows parity. Xvfb CI captures ordinary test windows through the real
X11 backend in an isolated HOME/XDG profile. Its package, pixel, clipboard, and
persistence checks do not prove GNOME Wayland consent, real hardware, or
fractional-scale behavior. GNOME acceptance remains a separate gate.
