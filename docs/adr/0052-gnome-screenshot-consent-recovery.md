# ADR 0052: GNOME screenshot consent recovery

## Status

Accepted for the Ubuntu 24.04 GNOME Wayland capture path.

## Context

A fresh installed Kiri session can receive Screenshot portal response code 2
(`Other`) from a desktop shortcut without showing the user a permission prompt.
The same code is returned after a user explicitly denies the real GNOME prompt
in the isolated Wayland CI desktop. The portal therefore does not tell Kiri
whether the user denied access or GNOME could not present the prompt.

Ubuntu 24.04 exposes Screenshot portal version 2. Its interactive picker can
return a window or a selected area, while Kiri's frozen capture requires a
whole-display image for its overlay coordinates. Switching to the interactive
picker would not preserve that capture contract.

## Decision

Keep one noninteractive whole-display Screenshot request. Treat response code 2
as an ambiguous access failure and show a localized recovery message: if no
system prompt appeared, bring a window to the front and try Capture again.
Do not automatically open a second permission dialog after response code 2;
that would also happen when the user chose Deny.

Keep portal errors in diagnostics free of captured pixels and OCR content.
The native GNOME Wayland checklist still requires a real allow, cancel, and
retry test before claiming first-use desktop acceptance.

## Consequences

The user gets a concrete retry path instead of the raw `Other` response. This
does not grant screenshot access or guarantee that every GNOME session will
show its first consent dialog. A future portal version that supports an
explicit whole-screen target may allow a different flow after native testing.
