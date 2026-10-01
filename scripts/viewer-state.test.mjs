import assert from "node:assert/strict";
import test from "node:test";

import {
  createViewerLoadingState,
  createViewerReadyState,
  viewerMediaKind,
  viewerStateAfterFailure,
} from "../src/windows/viewer-state.js";
import { installViewerCopyShortcut } from "../src/windows/viewer-copy-shortcut.js";

const video = { kind: "video" };
const image = { kind: "image" };

test("unresolved viewer content never renders as image or video", () => {
  assert.equal(viewerMediaKind(createViewerLoadingState()), null);
  assert.equal(viewerMediaKind(viewerStateAfterFailure(null, "ready")), null);
});

test("viewer renders media only after asset metadata resolves", () => {
  assert.equal(viewerMediaKind(createViewerReadyState(video)), "video");
  assert.equal(viewerMediaKind(createViewerReadyState(image)), "image");
});

test("viewer distinguishes missing, unreadable, and playback failures", () => {
  assert.equal(viewerStateAfterFailure(video, "missing").kind, "missing");
  assert.equal(viewerStateAfterFailure(video, "unreadable").kind, "unreadable");
  assert.equal(
    viewerStateAfterFailure(video, "libraryUnavailable").libraryUnavailable,
    true,
  );
  assert.equal(viewerStateAfterFailure(null, null).availabilityUnknown, true);
  assert.equal(viewerStateAfterFailure(video, null).kind, "playbackFailed");
  assert.equal(viewerStateAfterFailure(video, "ready").kind, "playbackFailed");
});

function copyShortcutFixture() {
  let handler;
  let copies = 0;
  let canCopy = true;
  let hasTextSelection = false;
  const target = {
    addEventListener(name, listener) { assert.equal(name, "keydown"); handler = listener; },
    removeEventListener(name, listener) { assert.equal(name, "keydown"); assert.equal(listener, handler); handler = null; },
  };
  const dispose = installViewerCopyShortcut(target, {
    canCopy: () => canCopy,
    hasTextSelection: () => hasTextSelection,
    copy: () => { copies += 1; },
  });
  return {
    press(overrides = {}) {
      const event = { key: "c", metaKey: false, ctrlKey: false, altKey: false, shiftKey: false,
        repeat: false, defaultPrevented: false, target: { closest: () => null },
        preventDefault() { this.defaultPrevented = true; }, ...overrides };
      handler(event);
      return event.defaultPrevented;
    },
    get copies() { return copies; },
    canCopy(value) { canCopy = value; },
    selectText(value) { hasTextSelection = value; },
    dispose,
  };
}

test("viewer Cmd/Ctrl+C copies once and suppresses key-repeat", () => {
  const fixture = copyShortcutFixture();
  assert.equal(fixture.press({ metaKey: true }), true);
  assert.equal(fixture.press({ ctrlKey: true, key: "C" }), true);
  assert.equal(fixture.press({ metaKey: true, repeat: true }), true);
  assert.equal(fixture.copies, 2);
  fixture.dispose();
});

test("viewer copy keeps native text, dialog and IME behavior", () => {
  const fixture = copyShortcutFixture();
  for (const overrides of [
    { target: { closest: () => ({ tagName: "INPUT" }) } },
    { target: { closest: () => ({ role: "dialog" }) } },
    { isComposing: true }, { keyCode: 229 }, { defaultPrevented: true },
    { altKey: true }, { shiftKey: true },
  ]) fixture.press({ metaKey: true, ...overrides });
  fixture.selectText(true);
  assert.equal(fixture.press({ metaKey: true }), false);
  assert.equal(fixture.copies, 0);
  fixture.dispose();
});

test("viewer copy ignores unresolved, busy or editing content", () => {
  const fixture = copyShortcutFixture();
  fixture.canCopy(false);
  assert.equal(fixture.press({ metaKey: true }), false);
  fixture.canCopy(true);
  assert.equal(fixture.press(), false);
  assert.equal(fixture.press({ metaKey: true, key: "v" }), false);
  assert.equal(fixture.copies, 0);
  fixture.dispose();
});
