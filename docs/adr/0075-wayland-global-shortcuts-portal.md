# ADR 0075: Opt-in Wayland GlobalShortcuts Portal

## Status

Implemented; installed-package/compositor acceptance remains tracked in
[the Linux guide](../linux.md#global-shortcuts-portal-acceptance).
Extends the shortcut decision in [ADR 0051](0051-ubuntu-linux-capture-boundaries.md)
without changing its command fallback or capture/recording constraints.

## Context

Ubuntu 24.04's GNOME 46 backend does not implement GlobalShortcuts. GNOME added
it in 48; KDE and Hyprland also have implementations. A desktop name or a
Wayland environment variable does not prove that the selected Portal backend
supports this interface. BindShortcuts can return only part of the requested
set, and a session can end or its bindings can change independently of Kiri.

## Decision

- Keep native X11/macOS/Windows capture registration and all three Linux CLI
  actions. Never write compositor configuration, GNOME custom bindings, or
  FIFO listeners.
- Add a Wayland-only Settings card. Read the actual D-Bus interface version
  with no default/cached success. Setup is explicit, restricted to the focused
  library window, and asks for exactly `capture`, `pause-resume`, and `stop`.
  The desktop chooses keys; Kiri does not force a preferred trigger.
- A dedicated D-Bus connection first registers the host application identity
  `io.yuxino.kiri` with `org.freedesktop.host.portal.Registry`. The `.deb`
  includes the matching hidden `io.yuxino.kiri.desktop` metadata entry; the
  existing `kiri.desktop` launcher and icon are unchanged. A missing or failed
  host registration fails closed to command guidance. This is a host `.deb`
  implementation, not a new Flatpak packaging promise.
- Subscribe before making requests, verify request/session handles, bound
  noninteractive calls and approval waits, and close pending requests and
  sessions on cancellation or error. Only the resident single-instance process
  starts the actor. One actor owns one session and serializes operations.
- Render only the current returned `trigger_description`; missing, empty,
  duplicate, or unknown IDs never become active shortcuts. Re-list on
  ShortcutsChanged and Settings focus. Clear visible bindings on refresh,
  session closure, service-owner loss, and protocol/call failures. Revisions
  keep delayed IPC replies from replacing newer lifecycle state.
- Route approved activations through the existing capture/recording actions;
  ignore other sessions/IDs and suppress held-key repeats until Deactivated.
- Successful explicit setup with at least one binding opts in to one restore
  attempt at process startup, disclosed in the card. Denial/failure does not
  loop. Session/service loss requires explicit Reconnect; revocation of all
  bindings disables automatic restoration. Disconnect closes Kiri's session
  and disables restoration, but does not erase the desktop's saved choices.
- Prefer ConfigureShortcuts on version 2. Version 1 retains desktop settings
  guidance and explicit new-session setup. Empty parent-window identifiers are
  protocol-valid; never forge an exported Wayland handle or pass an XWayland
  XID as a Wayland parent. Focus/transient-dialog behavior needs compositor QA.

## Verification boundary

Portable state and React handler tests are separate from isolated D-Bus wire
contract tests. The latter run the production client against an in-process
service on a private daemon, without a desktop or user permission store.
Neither proves keyboard delivery, approval UI, app metadata presentation, or
recording stop on an installed GNOME/KDE/Hyprland package. See the acceptance
matrix before closing #47.
