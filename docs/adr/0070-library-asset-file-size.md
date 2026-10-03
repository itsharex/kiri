# ADR 0070: File size on library asset cards

Status: Accepted

## Decision

Image, video, and GIF library cards show the current shareable file size beside
pixel dimensions on a full-width metadata row, including in Trash. Use decimal B, KB, MB, GB, and TB units
with at most one decimal place. The existing metadata tooltip includes the size
when a narrow card truncates the line. Missing or invalid files omit the size;
a real empty file displays 0 B. Pending recordings have no size until saved.

Read filesystem metadata on a background worker when listing assets. Do not
read media contents or persist size in `library.json`; normal library refreshes
pick up saved edits and replacements. Reject directories and symlinks. This is
the media file size, excluding thumbnails, clean sources, and editing projects.
