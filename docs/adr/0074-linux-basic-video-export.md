# 0074 — Bounded native Linux video cuts

Status: accepted for implementation; packaged Ubuntu/GNOME acceptance remains open.

## Context

Issue #74 requests accurate normal-speed cuts, deleting middle clips, reordering
retained clips, and exporting a separate MP4 while retaining source audio.
The existing project, viewer ownership, export snapshot, cancellation and library
commit boundaries already provide the shared application contract. Enabling the
entire macOS/Windows editor would promise unsupported privacy masks and effects.

## Decision

Use only installed GStreamer libraries. Each cut decodes from an accurately located preceding keyframe, clips raw
video/audio to the exact requested interval, and rebases timestamps onto the
cumulative output clock. A one-frame lookahead retains a long-held frame across
a cut even when a decoder reports only its nominal frame duration. Audio seeks
accurately to the requested interval before sample-level clipping.
Independent bounded video/audio workers feed one continuous H.264/AAC encoder.
A new encoder per clip would accumulate AAC delay at joins; compressed-stream
cuts would be inaccurate between keyframes. Both are avoided.

Video orientation is applied before scaling. Output uses even H.264 dimensions,
never enlarges small sources, and supports the existing original/share/small
size presets. Variable-rate and held frames retain their presentation intervals; the
last raw frame is clipped to the exact cut end. Audio is normalized to 48 kHz stereo AAC with the installed `avenc_aac` plugin;
the selected AAC-LC encoder's 1024-sample priming is compensated at the mux pad
so it does not shift sound or truncate the tail. Source timing gaps become
silence, with a cumulative sample clock to avoid
per-clip rounding drift. No source audio track means no output audio track.

The initial supported source is one progressive square-pixel video track and zero or one audio
track, without subtitle tracks, at most 8192 pixels per edge and 32 megapixels.
Unsupported track layouts, interlacing, missing codecs, undecodable or incomplete
ranges fail visibly. They must not silently omit tracks or pictures.

Expose separate editing, speed, effects, annotation and export-size capabilities.
Linux opens cuts/reordering and size presets only when the required system plugins
are available. Speed changes, effects (including privacy masks), annotations and
stickers are rejected by the backend. A saved project using unavailable operations
remains intact and read-only; previewing the original does not replace its draft.
Viewing-only playback speed is independent of export speed.

Every pipeline returns to NULL through RAII. Bounded input/output queues, polled
cancellation, and stalled-decode/finalization deadlines cover all work phases.
The existing export wrapper owns temporary files; source snapshots, matching-viewer
checks, single-export ownership, library identity/generation checks and the atomic
cancel-versus-library-import boundary remain unchanged. No personal files are
modified or deleted by validation tests.

## Validation and limits

Native tests encode real MP4 inputs, decode every exported frame and audio sample,
check non-keyframe cuts, order, boundary colors, audio tones/synchronization,
raw-versus-encoded waveform correlation at joins (within two milliseconds),
static/VFR timing, silent inputs, rotation, invalid requests and cancellation.
Frontend tests cover default-deny capabilities and unsupported-draft preservation.

These tests establish the source/media path on their recorded Linux runtime.
They do not establish exact-package Ubuntu 24.04/GNOME native UI acceptance,
close/cancel interaction, library migration races, or physical hardware coverage.
Use `docs/qa/linux-video-export.md` for the remaining installed-app checks. Issue
#74 stays open until that evidence is recorded; this is not full feature parity.
