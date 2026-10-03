# Security policy

Please do not open a public Issue for a vulnerability involving capture
contents, local file access, permissions, or unintended data disclosure.

Report security issues privately through GitHub Security Advisories for this
repository. Include the affected revision, reproduction steps, and expected
impact. Remove personal screen contents, tokens, and credentials from logs or
sample files.

kiri is local-first and does not upload captures automatically. A change that
transfers capture contents or other user data must make the destination and
user action explicit.

Editable annotation documents are treated as untrusted local input. Kiri
validates their schema and bounds, verifies the source and flattened-image
hashes, and refuses stale editor saves instead of pairing a document with a
different image revision. Native Save panels issue a one-time destination
authorization; unrestricted filesystem paths are not exposed to the WebView.

Recording, paused-segment merging, video thumbnails, and explicit GIF
conversion use operating-system media frameworks. macOS uses AVFoundation and
ImageIO; Windows uses Media Foundation and Windows imaging components; Linux
uses system-installed GStreamer plugins. These operations do not download or
execute a third-party media binary, and media bytes remain on the device.

Linux video playback has a process-scoped loopback HTTP capability. It binds
only to `127.0.0.1`, uses a random port and 128-bit token, validates Host and any
supplied Origin, and resolves canonical IDs only to active video assets. Only
GET/HEAD are accepted. Headers, byte ranges, buffers and concurrency are bounded;
there are no mutations or arbitrary filesystem routes. The media-only CSP
exception does not permit general WebView network requests. The endpoint stops
at application exit.

Application updates use Tauri's signed updater with a fixed HTTPS manifest and
an embedded public key. Checks, downloads, and installation are separate
user-initiated actions. A downloaded archive must pass signature verification
before installation; Kiri never accepts an updater endpoint, executable path,
or public key from remote release notes.

Optional remote OCR follows that rule per image. The app prepares only the
selected region locally, discloses the configured provider origin, model, and
image size, and sends it only after the user activates a visible Send or Retry
action. Return uses local OCR for that image. Failed requests are never retried,
rerouted, or uploaded through a fallback provider automatically.

Remote OCR API keys are write-only inputs stored in macOS Keychain, Windows
Credential Manager, or Linux Secret Service. They must not appear in profile
JSON, IPC responses, logs, screenshots, crash reports, fixtures, or
environment-variable fallbacks.
