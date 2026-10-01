# ADR 0064: Direct QR selection, saving and opening

- Status: Accepted
- Date: 2026-10-02
- Supersedes: ADR 0061's additional link confirmation and explicit-only favorites

## Decision

Capture recognition places QR center markers directly over the original frozen
region. It does not show another image preview in a large modal. Selecting a
marker reveals a compact nearby content panel that stays inside the display.
Closing or pressing Escape returns to the existing region and annotations.
Saved images can still use a result dialog where no capture surface exists.

Results use simple Link, Text and WeChat labels. Ordinary HTTP links do not get
a danger warning merely because of their scheme. Keep the existing validation
that limits opening to explicit HTTP/HTTPS URLs without credentials, whitespace,
control characters or backslashes. Custom WeChat schemes remain readable and
copyable content. Circular Mini Program recognition remains out of scope.

A successfully decoded code is saved to QR Favorites when the user selects its
marker. Do not save an entire batch automatically. Repeated payloads reuse the
existing record; unreadable codes are not saved. Users may remove the favorite,
and saving failures remain visible with a manual retry. Before opening, finish
all pending saves for previously selected codes and prevent further selection.
Closing the results does not revoke a save already accepted for a selected code;
the original library identity and generation must still match when importing.

Open Link is one explicit action. It delegates to the system default browser
without a second confirmation. A successful capture action ends the owned
capture; restore the pre-capture windows before requesting the browser, then
close the overlays without restoring focus again. A failed request restores
the overlay and keeps the result available. Saved-image dialogs close after
successful opening; the library's favorites view stays available.

Long payloads wrap and scroll in a bounded content area. Keep actions outside
that area so a long link does not hide the opening and copying controls.

## Verification

Tests cover one-click opening, pending-save ordering, repeated-content reuse,
failed saves and opening, rapid clicks and stale result callbacks. Isolated
browser checks cover pointer activation, long payload scrolling and content
classification. Native browser handoff and capture focus need separate device
acceptance; a mocked browser action does not prove that handoff.
