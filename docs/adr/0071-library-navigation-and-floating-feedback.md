# ADR 0071: Library navigation and floating feedback

Status: Accepted

## Decision

Changing the media tab, favorites, tag, search, or Library/Trash destination
returns the capture grid to the top before painting. Background refreshes within
one view preserve the current position.

Import, clipboard paste, and other transient library results share a compact
bottom-centered toast. Feedback never inserts a row or shifts the header/grid.
Success notices last three seconds; failures last six. New feedback replaces old
feedback and gets its own timer. Text wraps within narrow windows. Keep the
existing monochrome tokens, restrained border, accessible status/error roles,
and reduced-motion support. GIF progress uses the same floating surface while
processing. Recovery actions remain available in their existing error surfaces.
