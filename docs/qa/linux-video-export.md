# Linux basic video export acceptance (#74)

## Automated native-media evidence

Run against the exact source revision with the installed system GStreamer plugin
set, retaining only generated test fixtures in a new empty evidence directory:

```bash
KIRI_LINUX_VIDEO_QA_DIR=/path/to/new/evidence \
  cargo test --locked --manifest-path src-tauri/Cargo.toml \
  video_export::linux_export -- --test-threads=1
pnpm test:release-tools
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml --all-targets
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets
git diff --check
```

Retain command logs, source SHA, system/GStreamer/plugin versions, source/output
hashes, decoded-frame and audio assertions, and any failed fixtures. Color and
tone fixtures test the real native media pipeline; they do not replace desktop QA.

## Exact installed Ubuntu 24.04/GNOME package

Record the candidate commit, CI run/artifact, `.deb` SHA-256, package version,
installed executable hash, desktop/compositor/backend and GStreamer versions.
Use a disposable HOME/XDG library and generated fixture inputs. Never manipulate
an existing personal capture library directly. A browser preview or an unpackaged
Cargo executable is not this acceptance.

1. Import silent and audible MP4/MOV fixtures. Confirm playback remains available.
   Open Trim & Export. Only trim, split, delete, reorder and size presets appear;
   export-speed, mask, zoom, annotation and sticker tools do not appear. Viewing
   playback speed still changes only playback.
2. Cut at non-keyframe fractional boundaries; split/delete a middle clip; reorder
   retained clips with mouse and keyboard; undo/redo; close/reopen the saved draft.
   Export each preset. Verify original bytes and draft survive and a new asset is
   imported once. Fully decode output, inspect boundary frames and dimensions,
   measure duration and audible clip order/sync. Include static long frames,
   variable frame rate, rotation metadata, and a source with delayed/shorter audio.
3. Reopen a project created on another platform with speed, privacy mask, zoom,
   annotation or sticker content. Confirm an explicit unsupported-edit notice,
   original playback, disabled editing/export and byte-identical saved project.
   Do not “solve” this by dropping its private mask or annotations.
4. Cancel during source copy and while actual video/audio encoding is active.
   Repeated Cancel must remain safe; progress is monotonic and reflects processing.
   Confirm no new library item, partial MP4 or staging directory; retry succeeds.
   Cancel after final saving has begun must accurately report that saving won.
5. Close the viewer while exporting and while a draft save is pending. Follow the
   existing close guard. Check no stale event changes a newer viewer, no unintended
   import after a successful cancel and the latest successfully saved draft remains.
6. Start a second export from the same/different viewer; one active export owns the
   slot. Exercise missing source, damaged MP4, missing codec, unsupported extra
   audio/subtitle tracks, invalid segment and an unwritable temporary destination.
   Every failure must be visible, leave source/draft intact and release ownership.
7. While export is active, use the app's supported library-change workflow or
   recoverable source Trash. Confirm a library identity/generation change prevents stale import. Record
   source Trash behavior separately: the existing export owns its source snapshot
   and does not recheck source Trash/replacement at final import. Restore via the
   app, then retry. Do not edit library files behind Kiri.
8. Close/reopen Kiri. Verify new MP4 playback and draft persistence; check English,
   Simplified Chinese and Japanese notices. Save screenshots and output audits.

## Claim boundaries

A native-media pass is distinct from the installed-app checklist above. Xvfb is
not GNOME Wayland, and virtual devices are not physical audio hardware. Keep
unrun, failed and passed checks separate. Source audio in an imported video does
not establish that Linux recording can capture microphone/system audio (#73).
Do not close #74 or advertise full macOS/Windows editing parity from unit/media
coverage alone.
