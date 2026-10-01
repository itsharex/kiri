# ADR 0067: Bounded local QR recovery

- Status: Accepted
- Date: 2026-10-02
- Extends: ADR 0062's full-image contrast passes

## Context

Dense montages can be readable when each code is selected alone but incomplete
when scanned together. The current Quirc decoder limits each scan to 32 finder
patterns and 8 grids. Neighboring codes can also consume those slots with
invalid cross-code grids, even though the geometry validator rejects them.

## Decision

Keep the original grayscale and contrast scans. When several finder patterns
remain or the decoder reaches its finder limit, recover readable codes from
local views. At the finder limit, cover the whole image before spending the
remaining budget on overlapping views, so the first observed finders do not
monopolize the scan.

Direct grayscale sampling targets 500,000 pixels per local view and caps total
additional sampled/decoded pixels at 8,000,000. Ordinary single-code and blank
images skip local recovery. Existing 20 MiB PNG, 32 MP image and 64 physical-code
limits still apply.

Local recovery adds only decoded, fully contained candidates and retains the
finder-geometry validation. Map each candidate back into the original image
before merging by physical position. Equal payloads at different locations
remain separate center markers. This does not add support for circular Mini
Program codes or guarantee every stylized, damaged or undersized code is readable.

## Verification

Generated 3×3 and 4×4 montages exercise the decoder's whole-image limit and
neighboring-finder interference. A 32 MP fixture checks recovery and coordinates
throughout the image. Public Google image-search screenshots are temporary QA
inputs, not committed fixtures or evidence that every visible symbol is a QR
code. Compare the same image before and after, and distinguish a whole-image
miss from a code that also fails when cropped alone.
