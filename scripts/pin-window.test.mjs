import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { createLibraryHarness, deferred, nodes, settleRequests } from "./helpers/library-render-harness.mjs";

const source = 'import React from "react";\n' + readFileSync(new URL("../src/windows/PinWindow.tsx", import.meta.url), "utf8");

function pinWindow() {
  const requests = [];
  let pinned, reads = 0;
  const window = {
    // The Linux window manager can still be applying the builder's true state.
    isAlwaysOnTop: async () => { reads += 1; return false; },
    setAlwaysOnTop(value) { const result = deferred(); requests.push({value, ...result}); return result.promise; },
  };
  const h = createLibraryHarness({}, source, {modules: {
    "@tauri-apps/api/window": {getCurrentWindow: () => window},
    "../lib/ipc": {mediaUrl: () => "image:test", onAssetContentChanged: async () => () => {},
      onPinOnTop: async callback => { pinned = callback; return () => { pinned = null; }; }},
    "./pin-window.css": {},
  }});
  const component = h.mount("PinWindow", {id: "test-image"});
  component.render();
  return {component, requests, reads: () => reads, pinned: () => pinned()};
}

const pinButton = component => nodes(component.render()).find(node => node?.type === "button");
const pinLabel = component => pinButton(component).props.children[0];

test("a fresh pin keeps its builder state despite a transient false native snapshot", async () => {
  const h = pinWindow();
  await settleRequests();
  assert.equal(pinLabel(h.component), "Unpin");
  assert.equal(h.reads(), 0);
  pinButton(h.component).props.onClick();
  assert.equal(h.requests[0].value, false, "the first click must actually unpin");
  h.requests[0].resolve();
  await settleRequests();
  assert.equal(pinLabel(h.component), "Pin on Top");
});

test("a reused pin event restores the button and the next click unpins", async () => {
  const h = pinWindow();
  await settleRequests();
  pinButton(h.component).props.onClick();
  h.requests[0].resolve();
  await settleRequests();
  assert.equal(pinLabel(h.component), "Pin on Top");
  h.pinned();
  assert.equal(pinLabel(h.component), "Unpin");
  pinButton(h.component).props.onClick();
  assert.equal(h.requests[1].value, false);
});

test("a library repin wins over the completion of an older pending unpin", async () => {
  const h = pinWindow();
  await settleRequests();
  pinButton(h.component).props.onClick();
  h.pinned();
  h.requests[0].resolve();
  await settleRequests();
  assert.equal(pinLabel(h.component), "Unpin");
  pinButton(h.component).props.onClick();
  assert.equal(h.requests[1].value, false);
});

test("failed native changes keep the pin state and rapid repeated clicks make one request", async () => {
  const h = pinWindow();
  await settleRequests();
  const click = pinButton(h.component).props.onClick;
  click(); click();
  assert.equal(h.requests.length, 1);
  h.requests[0].reject(new Error("unavailable"));
  await settleRequests();
  assert.equal(pinLabel(h.component), "Unpin");
  assert.ok(nodes(h.component.render()).some(node => node?.props?.role === "alert"));
});


test("a stale rejected toggle does not show an error after a successful library repin", async () => {
  const h = pinWindow();
  await settleRequests();
  pinButton(h.component).props.onClick();
  h.pinned();
  h.requests[0].reject(new Error("superseded"));
  await settleRequests();
  assert.equal(pinLabel(h.component), "Unpin");
  assert.ok(!nodes(h.component.render()).some(node => node?.props?.role === "alert"));
});
