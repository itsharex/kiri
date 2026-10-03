# Focused Linux residual repair acceptance

Refs [#82](https://github.com/yuxino/kiri/issues/82), following merged
[#83](https://github.com/yuxino/kiri/pull/83). This record does not close the issue
or qualify a release. The numbered cases below refer to the independent
23-case post-#83 retest, not the six headings in the original issue body.

## Repairs

- **Case 1, wrapped text handle resizing:** preview, hit bounds and saved bounds
  now use the same measured line wrapping as rendering. Fixed-edge/center
  anchoring is preserved, and a wrap-induced extra line is fitted within the
  available image height. A no-op pointer gesture remains a no-op history entry.
- **Case 8, fresh pin state:** a newly created always-on-top window no longer
  takes a transient window-manager getter result as its permanent initial UI
  state. Successful pin actions update state, synchronous busy protection
  prevents duplicate requests, and a newer library repin event takes precedence
  over an older in-flight response.
- **Linux WebKitGTK ownership:** the IPC signal captures a weak WebView instead
  of completing the WebView → manager → closure → WebView strong-reference cycle.
  Callback entry upgrades it temporarily; disposed receivers return before view
  access. The existing IPC request and registration logic are unchanged.

No manual unref, diagnostic instrumentation, process/cache tweak, security
bypass, or sandbox change is included. The Wry change is a local patch, not an
upstream backport.

## Source and package identity

The accepted combined candidate was built on
`264f687cb9119e57585d172ddbf90f2c16070d4a` with the seven-file repair patch. The
publication branch preserves those seven files byte-for-byte and adds only the
portable QA harness, its Linux CI integration/scheduling tests, and this record.
The candidate's version override was external to source; no prerelease version
or machine-specific build path is added to application configuration.

- Candidate: `kiri_1.6.7-retest.combined1_amd64.deb`
- Package SHA-256: `1064bc127fc56c84eb0477fcad676b8a9296643adf8407a1c888a5255a8f3e2d`
- Packaged executable SHA-256: `310583e267f36b574fae3de1a08fdbbff582c9616fb7caa71fc96303bf11bebc`
- Reviewed seven-file patch SHA-256: `b56f96b392951c920e227b873fdf0fae0f879daa776abeb247708cf78b44224c`
- Frozen source/provenance archive SHA-256: `8fac62a41f9b3822332cb3dd0a83645fec31bc89df0f0e5905465d54eb31a4b1`

These identify local acceptance artifacts. They are not download links or
claims that the publication commit's future CI package has the same checksum.
Private profiles, raw logs and desktop captures are not published here.

## Completed checks on the reviewed candidate

- 275 JavaScript/release-tool tests, production frontend build, 316 Rust tests,
  all-targets Cargo check and diff whitespace check passed.
- Six exact-version GLib/GIO lifetime tests passed, covering live delivery,
  disposed/late callbacks, natural release, a strong-cycle negative control,
  and reentrant disposal. The production Wry source contract passed.
- Strict Clippy failed with the same 43 diagnostics as the pristine base;
  there were no added diagnostics. It was not a clean Clippy pass.
- An initial SDK-only Rust run lacked GStreamer's `x264enc` and failed four
  media tests. The unchanged source passed all 316 with the established native
  runtime and a separate GStreamer registry.

The portable harness now lives at [scripts/qa/ipc-lifetime](../../scripts/qa/ipc-lifetime/README.md).
Its six-case Rust source and lockfile are unchanged from the reviewed harness.
The added source guard binds it to the reviewed production IPC function and
application dependency identities. GitHub Actions executes it in the Linux job.

## Focused native acceptance, completed 2026-10-02 UTC

Environment: Debian 13.6, XFCE 4.20, X11, bundled WebKitGTK 2.54, software GL,
default security/compositing behavior, and an isolated test profile.

- Five ordinary editor open/close cycles created five distinct recorded
  renderer PID/start-time identities. All five were absent after their cycles.
- After 334.2 seconds of library-only quiet settling, one renderer remained.
  The four-process family PSS was 417,889 KiB (408.1 MiB), below its filtered
  487,858 KiB baseline. PSS coverage was 4/4 processes.
- Seeded malformed legacy text and fresh four-line text passed side-handle
  resize, last-line hit testing, Undo/Redo, Save and reopen.
- A fresh pin passed first Unpin, direct Unpin/Repin, and repinning while the
  library was minimized; native above-state agreed with the UI.
- Save/reopen IPC, pin close, a 700×400 screenshot Cancel, and reopening an
  editor after Cancel passed. Opening and closing a synthetic two-second MP4
  viewer passed; this was not a new full playback qualification.
- After functional checks, one renderer remained and all ten tracked
  non-library renderer identities were absent.
- Normal Quit returned exit status 0. None of 14 recorded process identities
  was live afterward: 12 absent and two exited zombies.
- The original profile's 193 files and 56 library records remained unchanged.
  No personal data or Trash was deleted.

Earlier matched A/B evidence supports the ownership diagnosis: the instrumented
strong-capture control retained an extra renderer beyond 345 seconds; the weak
candidate retired it and stayed at one renderer through 385 seconds. The final
combined package above was uninstrumented and received its own focused retest.

## Coverage limits

The earlier independent 23-case sweep of the original post-#83 candidate was
20 pass, one fail and two partial. This final combined acceptance was a focused
retest, **not another complete 23-case sweep**. In particular, case 20's remote
OCR confirmation remains untested; no credential or remote request was used.

Fresh-text corner/edge-fit and fractional wrap thresholds were exercised in the
earlier repair-only build, not repeated in the final combined package. Physical
GPUs, Ubuntu 24.04/GNOME, Wayland, real mixed-DPI hardware and other native
platforms are outside this local acceptance. The canonical Xvfb harness was
blocked locally by the environment's D-Bus socket restriction.

Finite cycles and a quiet interval establish bounded renderer-retirement
evidence, not universal absence of leaks or proof that no allocations remain.
The shared OOM counter stayed at 27 during the final native observations; this
does not identify or prove the victim of any earlier OOM event. CI results for
the publication commit must be read separately from these local-package results.
