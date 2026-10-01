import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { createLibraryHarness, nodes, settleRequests, deferred, testAsset } from "./helpers/library-render-harness.mjs";
import { captureToolbarPosition } from "../src/windows/toolbar-layout.js";
import { initialQrSelection, qrPolygon } from "../src/qr/selection.js";

const source = 'import React from "react";\n' + readFileSync(new URL("../src/qr/QrResults.tsx", import.meta.url), "utf8");
const code = (index, text = "https://example.org/kiri-safe") => ({ index, text, url: text.startsWith("https:") ? text : null, host: "example.org", suspicious: false, corners: [[.1,.1],[.4,.1],[.4,.4],[.1,.4]] });
const scan = codes => ({ requestId: "test-request", width: 1000, height: 500, imageUrl: "data:image/png;base64,test", codes });
const has = (tree, text) => nodes(tree).includes(text);
const button = (tree, text) => nodes(tree).find(n => n?.type === "button" && has(n, text));

test("screenshot toolbar recognition is explicit and does not complete the capture", () => {
  const overlay = readFileSync(new URL("../src/windows/OverlayWindow.tsx", import.meta.url), "utf8");
  const toolbar = overlay.slice(overlay.indexOf("function Toolbar("), overlay.indexOf("function ColorSwatch("));
  const harness = createLibraryHarness({}, `import React,{useState,useRef,useEffect,useLayoutEffect} from "react";
    import {t} from "../i18n"; import {KiriIcon} from "../components/KiriIcons";
    const TOOLS=[], COLOR_PRESETS=[], ACCENT="black", toolbarRowStyle={};
    const captureToolbarPosition=${captureToolbarPosition.toString()}; ${toolbar}; export {Toolbar,ToolButton};`);
  let scans = 0, captures = 0;
  const component = harness.mount("Toolbar", { selection:{x:100,y:100,width:140,height:160},bounds:{x:0,y:0,width:1000,height:700},
    tool:"select",appearance:{},canUndo:false,canRedo:false,canSetSize:false,disabled:false,
    onQr:()=>scans++,onDone:()=>captures++ });
  const target = nodes(component.render()).find(n => n?.props?.title === "Recognize QR Codes");
  assert.ok(target);
  assert.equal(scans, 0);
  target.props.onClick();
  assert.equal(scans, 1);
  assert.equal(captures, 0);
  component.unmount();
});

test("region selection and initial mode choice do not trigger QR recognition", () => {
  const overlay = readFileSync(new URL("../src/windows/OverlayWindow.tsx", import.meta.url), "utf8");
  const selection = overlay.slice(overlay.indexOf("function afterSelection"), overlay.indexOf("// --- render ---"));
  assert.doesNotMatch(selection, /runQr/);
  assert.doesNotMatch(overlay, /switchMode\("qr"\)/);
  assert.match(overlay, /phaseRef\.current === "qr-result"\) \{ closeQr\(\); return; \}/);
});

test("saved-image dialog reuses the asset id and cancels only its own request on close", async () => {
  const calls = [], closed = [];
  const harness = createLibraryHarness({scanQr:async (...args)=>{calls.push(args);return scan([code(0)]);},
    cancelQr:async id=>calls.push(["cancel",id])}, source);
  const component = harness.mount("QrAssetDialog", {asset:{...testAsset,id:"saved-image"},onClose:()=>closed.push(true)});
  component.render(); await settleRequests();
  assert.equal(calls.length, 1);
  const [request, region, id] = calls[0];
  assert.equal(region, null); assert.equal(id, "saved-image");
  nodes(component.render()).find(n => n?.props?.onClose).props.onClose();
  assert.deepEqual(calls[1], ["cancel", request]);
  assert.deepEqual(closed, [true]);
  component.unmount();
});

test("single code is displayed directly; multiple and empty scans await selection", () => {
  assert.equal(initialQrSelection([code(0)]), 0);
  assert.equal(initialQrSelection([code(0), code(1)]), null);
  assert.equal(initialQrSelection([]), null);
  assert.equal(qrPolygon(code(0).corners, 1000, 500), "100,50 400,50 400,200 100,200");
});

test("duplicate payloads retain independent clickable positions without auto actions", () => {
  const calls = [];
  const harness = createLibraryHarness({ qrAction: (...args) => { calls.push(args); } }, source);
  const component = harness.mount("QrResults", { scan: scan([code(0), {...code(1), corners:[[.6,.6],[.9,.6],[.9,.9],[.6,.9]]}]) });
  let tree = component.render();
  assert.ok(has(tree, "Choose a QR code in the image"));
  const targets = nodes(tree).filter(n => n?.type === "g");
  assert.equal(targets.length, 2);
  targets[1].props.onClick();
  tree = component.render();
  assert.equal(nodes(tree).find(n => n?.props?.code)?.props.code.index, 1);
  assert.deepEqual(calls, []);
  component.unmount();
});

test("opening waits for explicit review confirmation; cancel never invokes navigation", async () => {
  const calls = [];
  const harness = createLibraryHarness({}, source);
  const component = harness.mount("QrDetails", { code: code(0), action: async a => calls.push(a) });
  let tree = component.render();
  assert.deepEqual(calls, []);
  button(tree, "Open Link").props.onClick();
  tree = component.render();
  assert.ok(has(tree, "Only open links you trust."));
  assert.deepEqual(calls, []);
  button(tree, "Cancel").props.onClick(); component.render();
  assert.deepEqual(calls, []);
  button(component.render(), "Open Link").props.onClick();
  tree = component.render();
  const buttons = nodes(tree).filter(n => n?.type === "button" && has(n, "Open Link"));
  buttons.at(-1).props.onClick(); await settleRequests();
  assert.deepEqual(calls, ["open"]);
  component.unmount();
});

test("saving a repeated payload retains its favorite state when selecting another copy", async () => {
  const calls = [];
  const harness = createLibraryHarness({ qrAction: async (...args) => { calls.push(args); } }, source);
  const component = harness.mount("QrResults", { scan: scan([code(0), code(1)]) });
  nodes(component.render()).filter(n => n?.type === "g")[0].props.onClick();
  await nodes(component.render()).find(n => n?.props?.code).props.action("favorite");
  nodes(component.render()).filter(n => n?.type === "g")[1].props.onClick();
  const detail = nodes(component.render()).find(n => n?.props?.code);
  assert.equal(detail.props.code.index, 1);
  assert.equal(detail.props.saved, true);
  await detail.props.action("unfavorite");
  assert.equal(nodes(component.render()).find(n => n?.props?.code).props.saved, false);
  assert.deepEqual(calls.map(c => c[2]), ["favorite", "unfavorite"]);
  component.unmount();
});

test("damaged and unsafe codes cannot open; content can be copied and saved explicitly", async () => {
  const harness = createLibraryHarness({}, source);
  const broken = harness.mount("QrDetails", { code: {...code(0), text:null, url:null}, action: async()=>{throw Error("unexpected");} });
  const tree = broken.render();
  assert.ok(has(tree, "This QR code could not be decoded. Try a clearer image."));
  assert.equal(nodes(tree).filter(n => n?.type === "button").length, 0);
  broken.unmount();
  const calls=[];
  const unsafe = harness.mount("QrDetails", { code: {...code(0,"javascript:alert(1)"), suspicious:true}, action:async a=>calls.push(a) });
  assert.equal(button(unsafe.render(), "Open Link"), undefined);
  button(unsafe.render(), "Copy Text").props.onClick(); await settleRequests();
  button(unsafe.render(), "Save QR Code").props.onClick(); await settleRequests();
  button(unsafe.render(), "Remove Favorite").props.onClick(); await settleRequests();
  assert.deepEqual(calls, ["copy", "favorite", "unfavorite"]);
  unsafe.unmount();
});

test("empty result explains retry and performs no action", () => {
  const harness = createLibraryHarness({}, source);
  const component = harness.mount("QrResults", {scan:scan([])});
  assert.ok(has(component.render(), "No QR codes found. Try a clearer image or a larger selection."));
  component.unmount();
});

test("favorite search rejects a late previous response", async () => {
  const first=deferred(), second=deferred();
  const harness=createLibraryHarness({listQrFavorites:q=>q?second.promise:first.promise},source);
  const component=harness.mount("QrFavorites");
  component.render(); await new Promise(resolve=>setTimeout(resolve,5));
  nodes(component.render()).find(n=>n?.type==="input").props.onChange({target:{value:"new"}});
  component.render(); await new Promise(resolve=>setTimeout(resolve,170));
  second.resolve([{...testAsset,id:"new",qrText:"Newest QR content"}]);await settleRequests();
  assert.ok(has(component.render(),"Newest QR content"));
  first.resolve([{...testAsset,id:"old",qrText:"Old QR content"}]);await settleRequests();
  assert.ok(has(component.render(),"Newest QR content"));assert.equal(has(component.render(),"Old QR content"),false);
  component.unmount();
});
