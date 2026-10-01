# ADR 0062: Merge QR results across contrast thresholds

- Status: Accepted
- Date: 2026-10-01
- Supersedes: ADR 0061's empty-only contrast fallback

## Decision

Always scan the original grayscale image and the three fixed thresholds
(64, 128, 192). Merge all passes: a readable result does not establish that
other codes in the same image are readable at that threshold.

Match physical codes by center and edge size in source pixels, allowing small
threshold-induced shifts and corner-order changes. Payload equality never
merges codes at different positions. A readable candidate may replace a matched
undecodable result; later failures or conflicting payloads retain the first
readable result. Sort and reindex the final results for independent selection.

Keep the total limit of 64 physical codes, original dimensions and normalized
coordinates. The work remains bounded to four image passes with one reusable
binary buffer. All recognition and explicit-action boundaries of ADR 0061
remain in effect.

## Verification

Public generated fixtures cover a partial first pass and an initially empty
first pass that needs two different thresholds. Tests also cover duplicate
content at separate positions, unreadable-to-readable upgrades, blank and
damaged images, and the total result limit. These tests do not establish
recovery of obstructed codes or native acceptance of a user's original image.
