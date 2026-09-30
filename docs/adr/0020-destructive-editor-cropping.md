# ADR 0020: Destructive screenshot cropping

- Status: Accepted
- Date: 2026-08-29

## Context

The screenshot editor can preserve a clean source and re-editable marks, but it
previously had no way to remove unwanted pixels after capture. Exporting a
smaller flattened image alone would leave the original pixels in the editable
source and make the library dimensions inaccurate.

## Decision

The editor exposes Crop beside Select. Its frame moves and resizes with eight
handles. The pending frame stays in the original document coordinates when
switching to annotation tools; reopening Crop restores its frame and undo/redo
history. Cancel crop removes only the pending frame, preserving annotations;
Escape does the same while Crop is active. Save and Save As
both include the crop in their output, but only Save changes the library.

For Save, the WebView translates intersecting marks and drops marks fully
outside the frame. Rust validates that document, crops the exact
content-addressed clean source, persists the cropped rendered image, and
updates the indexed pixel dimensions. The same revision check and
ordinary-write rollback protect the source, document, rendered image, and
index.

Save As writes only the cropped export. It does not mutate the library asset or
its editable project.

## Consequences

- Pixels outside a saved crop are no longer retained in the editable source.
- Cropping an annotated image stays re-editable inside the new bounds.
- Canceling Crop or canceling Save As leaves the library unchanged.
- Crop undo/redo is active in Crop; annotation undo/redo stays independent in
  other tools. Switching tools never writes or destructively crops the source.
- The backend derives the source crop from the opened revision instead of
  trusting source bytes supplied by the WebView.
