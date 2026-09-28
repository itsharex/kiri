# ADR 0052: GNOME screenshot consent recovery

## Status

Accepted for the Ubuntu 24.04 GNOME Wayland capture path.

## Context

A fresh installed Kiri session can receive Screenshot portal response code 2
(`Other`) from a desktop shortcut without showing the user a permission prompt.
The same code is returned after a user explicitly denies the real GNOME prompt
in the isolated Wayland CI desktop. The portal therefore does not tell Kiri
whether the user denied access or GNOME could not present the prompt.

Installed Ubuntu 24.04 GNOME 46 testing showed that retrying with Files in the
foreground also fails: GNOME reports that only the focused app may show a
system access dialog. Kiri hides its Library before freezing the display, so
retrying from that window does not keep Kiri focused either. Merely asking the
user to focus another window is not a working recovery path.

Ubuntu 24.04 exposes Screenshot portal version 2. Native testing showed that
the interactive picker can take a screenshot but does not grant the persistent
permission needed by the noninteractive whole-display request. The permission
store remained empty after completing that picker, and retrying Capture still
failed. The portal frontend offers the persistent Allow dialog only for a
noninteractive request while the requesting app has focus.

## Decision

Keep one noninteractive whole-display Screenshot request for the actual capture.
Treat response code 2 as an ambiguous access failure and show a localized
recovery action in Kiri's Library. Only when the user selects Request Access
from that focused window, make one noninteractive portal request. GNOME can
then show its persistent Allow dialog while Kiri has focus. Discard the
resulting whole-display image and let the user explicitly retry the normal
capture. Never open a second permission dialog automatically after response
code 2, because that code also follows Deny.

Keep portal errors in diagnostics free of captured pixels and OCR content.
The native GNOME Wayland checklist still requires a real allow, cancel, and
retry test before claiming first-use desktop acceptance.

## Consequences

The user gets a visible authorization step instead of the raw `Other` response.
Kiri does not retain or import the authorization image. The dialog may still be
cancelled or denied, and a successful grant does not guarantee that every GNOME
session accepts the subsequent request. Keep the native allow, cancel, and
retry checklist open until verified.
