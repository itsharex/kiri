import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { createLibraryHarness, deferred, nodes, settleRequests } from "./helpers/library-render-harness.mjs";

const source = `import React from "react";\n${readFileSync(new URL("../src/settings/DockVisibilityRow.tsx", import.meta.url), "utf8")}`;
const switchButton = (tree) => nodes(tree).find((node) => node?.props?.role === "switch");

test("Dock switch loads the saved value and commits only after native success", async () => {
  const saved = deferred();
  const calls = [];
  const harness = createLibraryHarness({
    getDockVisibility: async () => ({ supported: true, visible: true }),
    setDockVisibility: (value) => { calls.push(value); return saved.promise; },
  }, source);
  const row = harness.mount("DockVisibilityRow", {});
  assert.equal(switchButton(row.render()).props.disabled, true);
  await settleRequests();
  const button = switchButton(row.render());
  assert.equal(button.props["aria-checked"], true);
  button.props.onClick(); button.props.onClick();
  assert.deepEqual(calls, [false], "the same interaction cannot dispatch twice");
  assert.equal(switchButton(row.render()).props.disabled, true);
  assert.equal(switchButton(row.render()).props["aria-checked"], true);
  saved.resolve(); await settleRequests();
  const changed = row.render();
  assert.equal(switchButton(changed).props["aria-checked"], false);
  assert.ok(nodes(changed).includes("Show in Dock"), "changing Dock presence keeps this settings row mounted");
  assert.equal(changed.props.className, "kiri-settings-card kiri-dock-row");
  row.unmount();
});

test("Dock setting failure retains the saved value and presents an error", async () => {
  const harness = createLibraryHarness({ setDockVisibility: async () => { throw new Error("write failure"); } }, source);
  const row = harness.mount("DockVisibilityRow", {});
  row.render(); await settleRequests();
  switchButton(row.render()).props.onClick(); await settleRequests();
  const tree = row.render();
  assert.equal(switchButton(tree).props["aria-checked"], true);
  assert.equal(switchButton(tree).props.disabled, false);
  assert.ok(nodes(tree).includes("Couldn't change Dock visibility."));
  row.unmount();
});

test("reopening settings reads an already-hidden native preference", async () => {
  const harness = createLibraryHarness({ getDockVisibility: async () => ({ supported: true, visible: false }) }, source);
  const row = harness.mount("DockVisibilityRow", {});
  row.render(); await settleRequests();
  assert.equal(switchButton(row.render()).props["aria-checked"], false);
  row.unmount();
});

test("load failure offers retry without guessing a writable preference", async () => {
  let calls = 0;
  const harness = createLibraryHarness({ getDockVisibility: async () => {
    if (++calls === 1) throw new Error("IPC unavailable");
    return { supported: true, visible: false };
  } }, source);
  const row = harness.mount("DockVisibilityRow", {});
  row.render(); await settleRequests();
  const tree = row.render();
  assert.equal(switchButton(tree), undefined);
  const retry = nodes(tree).find((node) => node?.type === "button" && nodes(node).includes("Retry"));
  retry.props.onClick(); await settleRequests();
  assert.equal(switchButton(row.render()).props["aria-checked"], false);
  row.unmount();
});

test("Windows and Linux settings neither render nor query the Dock preference", async () => {
  for (const userAgent of ["Windows NT 10.0", "X11; Linux x86_64"]) {
    let calls = 0;
    const harness = createLibraryHarness({ getDockVisibility: async () => { calls++; return { supported: true, visible: true }; } }, source, { navigator: { userAgent } });
    const row = harness.mount("DockVisibilityRow", {});
    assert.equal(row.render(), null);
    await settleRequests();
    assert.equal(calls, 0);
    row.unmount();
  }
});

test("native unsupported capability suppresses the row even with a macOS user agent", async () => {
  const harness = createLibraryHarness({ getDockVisibility: async () => ({ supported: false, visible: true }) }, source);
  const row = harness.mount("DockVisibilityRow", {});
  row.render(); await settleRequests();
  assert.equal(row.render(), null);
  row.unmount();
});
