# ADR 0072: Live annotation transform preview

Status: Accepted

## Decision

Moving, resizing, and adjusting endpoints replace the existing annotation in
each preview frame; do not draw a second copy while leaving the original behind.
Selection outlines and handles follow the preview. Preserve the mark's stacking
position and keep the original document unchanged until pointer release.
Cancellation restores the original; one completed gesture creates one undo step.

The shared annotation canvas applies this to pen, rectangle, line, arrow, text,
and all mosaic shapes in capture, saved-image editing, and video annotations.
Video receives the same replacement geometry in its live layer composition.
New strokes remain drawing drafts until committed. Saved screenshots continue
to use their clean source and editable projects from ADR 0016.
