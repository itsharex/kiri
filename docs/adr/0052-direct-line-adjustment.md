# ADR 0052: Direct adjustment of selected lines

- Status: Proposed
- Date: 2026-09-27

## Context

The annotation canvas selects a newly drawn line or arrow and displays its
endpoint handles. The line or arrow tool remains active, so the next pointer
gesture starts a new mark instead of moving the visible handles. This makes the
selection appear editable when it is not.

## Decision

While the matching line or arrow tool is active, a gesture on the selected
mark's endpoint moves that endpoint; a gesture on its stroke moves the mark.
A gesture elsewhere continues to draw a new mark. The Select tool retains its
existing behavior for all mark types. Endpoint hover uses a crosshair and
stroke hover uses a grab cursor so the available action is visible.

## Consequences

Users can correct the mark immediately without switching tools, while still
being able to draw consecutive lines or arrows. Only the selected mark is
editable through its drawing tool, avoiding accidental edits to other marks.
