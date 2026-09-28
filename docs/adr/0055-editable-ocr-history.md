# ADR 0055: Editable OCR history with original text

- Status: Proposed
- Date: 2026-09-27

## Context

Text History currently stores the recognition result on an immutable source
image record. A user must correct misread text again each time they copy it.

## Decision

Text History offers Edit, Save, and Cancel. Saving changes the record's
searchable and copyable text while preserving the source image. The first edit
also stores the recognition result as `ocrOriginalText`; later edits leave it
unchanged. Restore Original Text replaces the correction and clears the extra
field. Older records load with this optional field absent.

The command accepts the text that the editor opened and rejects stale saves.
Empty or oversized corrections are rejected before the library index changes.
The original and corrected text stay local and migrate with the library.

## Consequences

Search and Copy Text use the correction. Restoring the original remains
possible after multiple edits. A failed index write leaves both versions and
the image unchanged.
