# ADR 0069: Image fidelity and Linux video playback

Status: accepted

## Context

Issue #82 exposed differences between editable image previews, stored projects
and exports, together with WebKitGTK rejecting custom-scheme video sources.

## Decision

- PNG composition keeps source transparency unless a user adds an annotation
  background. Imported ICC-tagged RGB/gray images are converted to sRGB and
  tagged with an sRGB profile, so later native crops retain the same color meaning.
- A crop first creates the new clean source and translates the surviving marks,
  then renders its flattened PNG. First save and reopening therefore use the same
  sampling context for mosaic. Text backgrounds count toward crop intersection.
- Text input and Canvas share tab stops and content bounds. Keyboard changes to
  font size edit the selected text and enter the same undo history as pointer changes.
- Shared appearance writes contain only changed fields. The native preference
  lock merges them into the latest state and publishes an ordered update event.
- Linux video elements use a process-scoped HTTP media capability on
  `127.0.0.1` with a random port and token. The server accepts only canonical video
  asset IDs, GET/HEAD, and validated byte ranges; it never exposes filesystem paths.
  Headers, concurrent clients, range responses and streaming buffers are bounded.
  It stops with the application. Image/custom-protocol routes remain unchanged.
- The Linux CSP permits this loopback origin for media only. External uploads,
  remote requests, generic file serving and decoder protocol overrides are absent.

## Consequences

WebKitGTK can feed local video into its normal HTTP media decoder. Color
conversion happens once at import. Existing editable projects remain readable;
subsequent cropped saves use the corrected render order. Original imported files
are untouched. Compositor-dependent countdown focus and recording startup still
require native desktop acceptance; unit checks alone do not prove those behaviors.
