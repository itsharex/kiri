# ADR 0056: Copy selected library items to a folder

- Status: Proposed
- Date: 2026-09-27

## Context

The library can select multiple captures, but its batch actions only change
favorites or Trash. Collecting screenshots and recordings for a report requires
repeated individual saves.

## Decision

Export Selected asks for one destination folder and copies the current visible
image or media file for each active selection. Image annotation sidecars and
clean sources stay in the library. Originals and library metadata do not move.
Files with existing names get a numbered name; existing files are never
overwritten. Successful copies remain when another file fails, and the UI
lists failed filenames for retry. Canceling the folder picker changes nothing.

## Consequences

Batch export works offline and preserves the editable library source. The
folder picker is an explicit user action. Large copies run outside the library
lock so browsing remains responsive.
