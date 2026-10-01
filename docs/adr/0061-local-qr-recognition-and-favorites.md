# ADR 0061: Local QR recognition and favorites

- Status: Accepted
- Date: 2026-10-01
- Request: [Issue 55](https://github.com/yuxino/kiri/issues/55#issuecomment-5931506146)

## Decision

Recognize QR Codes is a screenshot toolbar tool. Opening an older screenshot in
the editor offers the same tool; the library menu also provides it. Region
selection itself does not start recognition. Recognition reads the selected
frozen source or the saved image, including when editor marks are still unsaved.
Recognition runs locally on a background worker using a portable Rust decoder.
When the default whole-image threshold locates no codes, recognition retries
three fixed contrast thresholds (64, 128, 192), stopping at a readable result.
This bounded fallback preserves original dimensions and polygon coordinates;
it does not reconstruct missing pixels, promise recovery of covered finder
patterns, or replace the default result when codes were already located.
The original selected image retains clickable polygons for each physical code,
including repeated payloads. A single code opens its content directly; multiple
codes wait for a selection. Located but undecodable codes show a failure message.
Empty results invite a clearer image or another region. Closing capture results
returns to the screenshot selection or annotation state without completing or
discarding the screenshot. Escape closes the QR tool first; a later Escape
cancels capture.

No result navigates, copies, or saves automatically. Copy, Open Link, and Save QR
Code are explicit actions. Opening requires an additional review of the full URL
and ASCII destination host. Only explicit HTTP/HTTPS URLs without credentials,
control characters, whitespace, or backslashes can be opened. HTTP, IP/local
hosts and internationalized domains receive a caution. This heuristic does not
establish whether a website is trustworthy.

QR Favorites stores each selected code's crop, quiet-zone margin, and searchable
payload in the managed local library. An optional `qrText` field preserves old
library decoding and remains absent from ordinary captures. Saving a duplicate
active payload reuses its record. Removing a favorite moves it to recoverable
Trash; restoring it makes it available again. Favorites offer content copying,
QR image copying, and explicit link review.

## Boundaries

Images and payloads never leave the device during recognition. There is no
remote fallback. Scans are bounded to 20 MiB PNG, 32 million pixels and 64
located codes; binary/non-UTF-8 payloads are shown as undecodable. One worker
holds the scanning permit at a time. A capture-scoped owner and request UUID
prevent canceled or superseded results from being published or saved. Closing
the result discards its transient image. Library identity/generation checks
protect saving during a library move.

Tests use public generated images and temporary libraries. A frontend harness
proves interaction logic only; native build, packaged capture, focus and
platform URL/clipboard behavior require separate verification.
