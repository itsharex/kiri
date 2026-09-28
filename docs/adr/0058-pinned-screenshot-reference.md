# ADR 0058: Pinned screenshot reference windows

- Status: Proposed
- Date: 2026-09-28

## Context

A saved screenshot is useful as a reference while working in another app, but
the library and editor require switching windows.

## Decision

An active screenshot's library menu can open one resizable reference window
above other windows. The window displays the saved flattened image through
the existing local media protocol, including annotations and mosaic. It has
explicit Unpin and Close actions. Unpin drops the topmost level but keeps the
reference open; the user can pin it again. Closing the window does not edit or
delete the library asset. Normal screenshot completion remains clipboard-first.

## Consequences

macOS and Windows can use the native topmost window level. X11 window managers
may honor it differently; Wayland compositors can reject or ignore a topmost
request. The UI reports a rejected toggle. Device acceptance across those
desktops is required before this draft is merged.
