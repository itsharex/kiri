# ADR 0073: Shared-clock Linux recording audio

Status: accepted for implementation; physical desktop acceptance is separate.

## Context

Issue #73 identified disabled Linux system sound, microphone, microphone check,
and MP4 audio tracks. Installed dependencies alone did not implement them.
ScreenCast portal access authorizes only screen streams and cannot authorize
microphone access or identify an output monitor.

## Decision

Use bounded asynchronous `libpulse` recording streams feeding GStreamer PCM
`appsrc` branches in the H.264/MP4 pipeline. Read-only introspection resolves
the default output's monitor by `monitor_of_sink` metadata and the default
non-monitor input. Connection uses `NOAUTOSPAWN`, a three-second deadline and a
local-service check. Before connecting, only an explicit local Unix socket
address or the standard XDG runtime socket is accepted; network addresses and
ambiguous server lists are rejected. Recording pins this server and device, uses corked startup,
bounded draining of ready mainloop work (so transport defers cannot starve timing
or control events), bounded open/timing operations and nonblocking
disconnect. This avoids the unbounded synchronous open/flush waits in `pulsesrc`.
It never starts a daemon, changes devices, grants access,
unmutes an input, or uses a downloaded media executable.

Static capability checks look only for required plugins. Device discovery and
opening happen only after an explicit recording or microphone-check request.
Both-disabled recordings open no sound connection. GIF disables audio only for
that session, preserving MP4 preferences.

Each selected source negotiates 48 kHz stereo PCM and verifies a 250 ms / 96 kB
native maximum. The appsrc and downstream non-leaky queue each have the same
byte bound. The live mixer reserves that same 250 ms latency window, so late
arrival of correctly timestamped native PCM is not mistaken for silence.
Signed read/write positions and device latencies from raw Pulse timing snapshots
map samples to the common monotonic video clock. Each snapshot's wall-clock
stamp is translated once; later reads advance by PCM sample count. The initial
snapshot is requested after that stream first reports readable PCM, because an
uncork acknowledgement alone does not establish active source timing. Pending
timing requests are polled without blocking and cancelled on teardown. The
UI-oriented interpolated latency API is not used: its startup clock can stop
while worker polling time advances, creating artificial gaps. Pre-origin PCM
is trimmed at sample boundaries, and future monitor samples remain unread.
`audiorate` tolerates 20 ms of timestamp jitter and corrects device clock drift.
Large timestamp jumps,
native overflow/holes and stalled sources fail closed. Inputs are mixed and
encoded into one 192 kb/s libav AAC track; its 1024-sample priming is represented
as MP4 decode preroll instead of shifting the content. Overrun, source discontinuity, a
rerouted or suspended source, a source timeout, or a pipeline failure invalidates the active
segment. Encoder failure is visible to the existing recording state machine.
The first accepted screen frame starts the pipeline, excluding chooser/startup
warmup from audio. Repeated held video frames keep the mux fed during static
screens without altering real-time duration.

MP4 verification checks expected video/audio track counts, dimensions, AAC
format and positive duration. Finalization publishes only validated staged
output. Pause freezes the video end boundary. Audio drains for at most 500 ms through
that boundary, discarding future monitor/post-stop samples, then disconnects.
Merge preserves compressed video; each AAC input is decoded to PCM to discard
its own preroll, concatenated and encoded once. Both concat branches use the
same explicit cumulative segment base and completed-segment duration, with
`concat` automatic base adjustment disabled so actual sample rounding cannot
override that boundary. Mux interleave buffering is disabled to avoid deadlocks against the bounded video queue.

Microphone checks use the same resolved input and asynchronous libpulse backend, last at
most five seconds, stop when the originating capture disappears, and never
save PCM. Missing/muted inputs fail rather than presenting a successful meter.

## Verification boundary

Unit/native-library tests cover source classification, unavailable/muted input,
audio-off, injected single and mixed tones, decoded AAC content/duration,
24-pause boundary/content alignment, overflow cleanup and an explicit 60-second clock run.
Private-server CI separately checks real libpulse capture/metering and bounded
cancellation while the owned sound service is stopped. These use
isolated generated data, never a synthetic mode in the application.

Ubuntu 24.04 GNOME X11 and Wayland hardware tests must independently cover
real output monitors and microphones, device loss/switching, permission denial,
restart, cancellation and long recordings. A cloud container without sound
hardware or permission to create a private Pulse socket cannot establish that
acceptance. Its GStreamer tests remain compatibility evidence only.

References: [issue #73](https://github.com/yuxino/kiri/issues/73),
[PulseAudio stream API](https://freedesktop.org/software/pulseaudio/doxygen/stream_8h.html),
[PulseAudio source metadata](https://freedesktop.org/software/pulseaudio/doxygen/structpa__source__info.html).
