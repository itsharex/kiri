# ADR 0065: Recording save status and file copy

- Status: Accepted
- Date: 2026-10-02

## Decision

Stopping a recording immediately publishes a transient saving job to the
library. Its neutral placeholder remains visible throughout native encoder
shutdown, merging, GIF conversion and import. Do not show a fabricated
percentage, expose incomplete files for playback, or persist placeholders as
captures. Jobs are independent of the live recording session: starting another
capture or reopening the library must not make an unfinished save disappear.

Retire the placeholder after refreshing the library and recording recovery
state. Successful imports become normal assets; failures use the existing
visible error and recoverable pending-recording flow. Finishing one background
job must not clear another job.

Images copy as clipboard pixels. MP4 and GIF copy as native operating-system
file items through library actions and the viewer. Cmd/Ctrl+C copies a focused
capture in the library or the original file in viewer preview mode; text fields
and selected text retain ordinary text copying. Editing an unsaved video does
not offer file copying as if it included the draft changes. Copy failures remain
visible. Clipboard paste support depends on the destination application.

## Verification

Regression checks cover overlapping save jobs, state after the live recording
returns to idle, placeholder filtering and replacement, and copy shortcuts that
preserve text editing. Native file-paste acceptance and recording finalization
require separate device checks; renderer mocks do not prove them.
