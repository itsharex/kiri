# ADR 0063: QR center markers and supported detection geometry

- Status: Accepted
- Date: 2026-10-01
- Supersedes: ADR 0061's clickable polygons and single-code automatic selection

## Decision

Show one monochrome circular arrow at each physical QR code's center in the
source image. Use the intersection of its diagonals so perspective does not
shift the marker away from the projected center. Single and multiple results
both wait for explicit selection before showing content. Native buttons retain
keyboard activation, accessible names and a visible selected state. Selecting
a marker never copies, saves or opens a link.

Before returning a detected grid, check that its projective denominator stays
positive throughout the code and that all three predicted finder shapes agree
with the observed finder corners. Allow three modules plus pixel rounding for
damaged but locatable codes. Neighboring thumbnails can otherwise supply an
unrelated finder pattern and produce a spurious polygon across the image.
Do not filter by aspect ratio, decode success or payload: stretched codes,
damaged codes and repeated content retain their existing behavior.

Ordinary square WeChat QR codes use the same local decoder. Circular Mini
Program codes are a separate format and are currently unsupported. Do not add
a cloud recognition fallback or upload images to work around this boundary.

## Verification

Public synthetic fixtures cover neighboring finder patterns, a projective pole,
WeChat URL shapes and a custom payment URI with small center marks. Custom
schemes remain copyable content and cannot be opened by Kiri. Geometry tests
retain rotated, perspective, stretched and damaged codes. Frontend tests cover
explicit single-code selection, independent repeated-code markers and no
automatic actions. These checks do not prove WeChat account operations or
recovery of every code in a user's original capture.
