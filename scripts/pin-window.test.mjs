import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { createLibraryHarness, deferred, nodes, settleRequests } from "./helpers/library-render-harness.mjs";

const source = 'import React from "react";\n' + readFileSync(new URL("../src/windows/PinWindow.tsx", import.meta.url), "utf8");

function pinWindow() {
  const initialState = deferred();
  let pinned;
  const window = {isAlwaysOnTop: () => initialState.promise};
  const h = createLibraryHarness({}, source, {modules: {
    "@tauri-apps/api/window": {getCurrentWindow: () => window},
    "../lib/ipc": {mediaUrl: () => "image:test", onAssetContentChanged: async () => () => {},
      onPinOnTop: async callback => { pinned = callback; return () => { pinned = null; }; }},
    "./pin-window.css": {},
  }});
  const component = h.mount("PinWindow", {id: "test-image"});
  component.render();
  return {component, initialState, pinned: () => pinned()};
}

const pinLabel = component => nodes(component.render()).find(node => node?.type === "button").props.children[0];

test("a reused native pin event wins over an earlier pending state query", async () => {
  const h = pinWindow();
  await settleRequests();
  h.pinned();
  assert.equal(pinLabel(h.component), "Unpin");
  h.initialState.resolve(false);
  await settleRequests();
  assert.equal(pinLabel(h.component), "Unpin");
});

test("the initial native query still reports an unpinned window without a newer event", async () => {
  const h = pinWindow();
  await settleRequests();
  h.initialState.resolve(false);
  await settleRequests();
  assert.equal(pinLabel(h.component), "Pin on Top");
});
