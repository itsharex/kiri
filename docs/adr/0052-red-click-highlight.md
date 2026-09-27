# ADR 0052: High-contrast recorded click highlight

- Status: Proposed
- Date: 2026-09-27

## Context

The optional recorded click ripple uses white rings. On light backgrounds the
click position is hard to see during playback. A user requested a more visible
red highlight that remains legible against both light and dark content.

## Decision

Use a saturated red halo, ring, and center with a narrow white center edge.
Keep the existing animation timing and window capture behavior. The ripple is
still opt-in and appears in the exported recording when enabled.

## Consequences

The click position is easier to find when reviewing a recording. Native video
acceptance must inspect frames over light and dark backgrounds before merge.
