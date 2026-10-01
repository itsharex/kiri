# ADR 0068: QR markers in the saved-image editor

- Status: Accepted
- Date: 2026-10-02
- Supersedes: ADR 0064's saved-image result dialog

## Decision

All image recognition entry points use the image already on screen. The library
menu opens that image in the editor and starts recognition; an existing editor
receives a targeted request. Do not create a second image preview or a large
results dialog. Loading, an empty scan and failures use a compact dismissible
status with retry available after a failure.

In the editor, scan the clean source from the exact opened editor revision.
Temporarily hide annotations and crop controls while showing center markers on
that same source. Keep the canvas, pending text, crop, undo history and dirty
state mounted and unchanged. Freeze editing and save shortcuts until recognition
ends. A revision or dimension mismatch cannot publish markers onto another image.
OCR image records may use this surface for read-only recognition, while editing
and annotation saving remain unavailable for those records.

Escape and Close leave QR mode and restore the draft. Opening a readable web
link uses the default browser and leaves the editor open. Native window closing
still uses the existing unsaved-edit guard. Capture recognition retains its own
successful-open behavior of ending the owned capture.

Plain text containing a colon or an embedded URL remains Text, with no link
warning or Open Link action. Standard WeChat schemes remain copyable WeChat
content. Only a complete validated HTTP/HTTPS URL can be opened. Suspicious
control characters retain a brief content warning.

## Verification

Renderer regressions cover request locking, original-image marker placement,
compact failure and retry, draft preservation, Escape, targeted entry and opening
without closing the editor. Backend tests cover exact revision/source ownership.
Installed-app acceptance must exercise the editor toolbar and library menu on the
same dense public image; generated fixtures alone do not establish native UI.
