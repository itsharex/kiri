# ADR 0057: Explicit clipboard image import

- Status: Proposed
- Date: 2026-09-28

## Context

Images copied from a browser or chat currently need a temporary file before
they can be imported for annotation or OCR.

## Decision

The library offers Paste Image and accepts Cmd+V or Ctrl+V while the capture
grid has focus. Inputs and editable text keep ordinary paste behavior. Kiri
reads the desktop clipboard only when this action is invoked; there is no
background clipboard watcher.

The image is bounded, normalized to PNG, and imported as an ordinary local
image asset using the existing media path. It can then use the existing editor
and local OCR actions. An empty, invalid, or oversized clipboard image does
not create a library record.

## Consequences

Users can work with copied images without creating temporary files. The source
application's clipboard content is not modified and no image leaves the device.
