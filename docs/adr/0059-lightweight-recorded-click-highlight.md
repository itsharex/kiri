# ADR 0059: Lightweight recorded click highlight

- Status: Accepted
- Date: 2026-09-30

## Context

ADR 0054 changed the optional recorded click highlight to red so it remained
visible on light footage. Its wide halo, filled ring, and outlined center made
the click mark too prominent in the finished video.

## Decision

Keep the red cue and existing 460 ms lifetime, but draw one thin, unfilled ring
and a small center point. Reduce their peak opacity. The highlight remains
opt-in and visible in the exported recording when enabled.

## Consequences

The click position stays identifiable with less of the underlying content
covered. Native acceptance should inspect recorded frames on light and dark
footage because CSS preview alone cannot prove the captured appearance.
