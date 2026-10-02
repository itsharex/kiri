import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import ts from "typescript";
import { createLibraryHarness, nodes } from "./helpers/library-render-harness.mjs";
import * as project from "../src/annotation/project.js";
import * as crop from "../src/annotation/crop.js";
import * as layout from "../src/annotation/text-layout.js";
import * as composition from "../src/annotation/text-composition.js";

const dataUrl = source => `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
const compile = source => ts.transpileModule(source, {compilerOptions: {
  module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022,
}}).outputText;
const geom = await import(dataUrl(compile(readFileSync(new URL("../src/annotation/geom.ts", import.meta.url), "utf8"))));
const geomUrl = dataUrl(compile(readFileSync(new URL("../src/annotation/geom.ts", import.meta.url), "utf8")));
const model = await import(dataUrl(compile(readFileSync(new URL("../src/annotation/model.ts", import.meta.url), "utf8")
  .replaceAll('"./geom"', JSON.stringify(geomUrl)))));
const source = readFileSync(new URL("../src/annotation/AnnotationCanvas.tsx", import.meta.url), "utf8");
const text = {kind: "text", id: 1, text: "first line\nsecond line", rect: {x: 40, y: 40, width: 180, height: 45},
  color: "white", background: "transparent", fontSize: 18};
const rectangle = {kind: "rectangle", id: 2, rect: {x: 100, y: 100, width: 100, height: 80}, color: "white", width: 3};
const appearance = model.DEFAULT_APPEARANCE;

function annotation(initialDocument, options = {}) {
  const exports = [], creations = [], ref = {current: null}, changes = [];
  function canvas() {
    const value = {width: 640, height: 360, drawCalls: [],
      getBoundingClientRect: () => ({left: 0, top: 0, width: initialDocument.canvas.width, height: initialDocument.canvas.height}),
      setPointerCapture() {}, releasePointerCapture() {},
      toBlob(callback) { callback(new Blob([new Uint8Array([1, 2, 3])], {type: "image/png"})); },
    };
    const context = {font: "", setTransform() {}, save() {}, restore() {},
      measureText(text) { const size = Number(this.font.match(/ ([\d.]+)px/)?.[1] ?? 18); return {width: text.length * size * .4}; },
      drawImage: (...args) => value.drawCalls.push(args)};
    value.getContext = () => context;
    creations.push(value);
    return value;
  }
  const live = canvas();
  const image = {complete: true, naturalWidth: initialDocument.sourcePixels.width, naturalHeight: initialDocument.sourcePixels.height};
  const harness = createLibraryHarness({}, source, {
    modules: {
      "./geom": geom, "./model": model, "./project.js": project, "./crop.js": crop,
      "./text-layout.js": layout, "./text-composition.js": composition,
      "./render": {textFont: size => `600 ${size}px sans-serif`, renderAll(r, marks) {
        if (r.exporting) exports.push({source: r.sourceImage, sourceWidth: r.sourceWidth, sourceHeight: r.sourceHeight,
          sourceOffset: r.sourceOffset, regionSize: r.regionSize, scaleX: r.scaleX, scaleY: r.scaleY,
          marks: structuredClone(marks), canvasWidth: r.ctx.canvas?.width});
      }},
    },
    document: {createElement: type => { assert.equal(type, "canvas"); return canvas(); }},
    globals: {devicePixelRatio: 1},
    attachRef(node) { if (node.type === "canvas") node.props.ref.current = live; },
  });
  const props = {ref, image, region: {x: 0, y: 0, ...initialDocument.canvas}, initialDocument,
    appearance, tool: "select", onHistoryChange() {}, onCancel() {},
    onDocumentChange: marks => changes.push(structuredClone(marks)), ...options};
  const component = harness.mount("default", props);
  component.render();
  return {component, props, ref, changes, exports, creations, live,
    pointer(name, x, y) {
      const node = nodes(component.render()).find(node => node?.type === "canvas");
      node.props[name]({clientX: x, clientY: y, button: 0, pointerId: 1, detail: 1,
        currentTarget: live, preventDefault() {}, stopPropagation() {}});
      component.render();
    },
  };
}

const documentWith = marks => ({schemaVersion: 1, canvas: {width: 640, height: 360}, sourcePixels: {width: 640, height: 360}, marks});

test("keyboard-style live font changes update the selected mark and commit one undoable edit", () => {
  const h = annotation(documentWith([text]), {selectedMarkId: text.id});
  h.ref.current.setTextFontSizeLive(32); // Keyboard input has no pointerdown.
  h.component.render();
  h.ref.current.setTextFontSizeLive(48);
  h.component.render();
  h.ref.current.endTextFontSizeAdjustment();
  h.component.render();
  assert.equal(h.changes.length, 1);
  const larger = h.changes[0][0];
  assert.equal(larger.fontSize, 48);
  assert.equal(larger.rect.width, text.rect.width * 48 / 18);
  assert.equal(larger.rect.height, text.rect.height * 48 / 18);
  h.ref.current.undo();
  h.component.render();
  assert.deepEqual(h.changes.at(-1), [text]);
});

test("font changes repair a previously saved short text box using explicit and wrapped lines", () => {
  const legacy = {...text, text: "long words wrap here\nsecond paragraph", fontSize: 48,
    rect: {x: 40, y: 40, width: 180, height: 45}};
  const h = annotation(documentWith([legacy]), {selectedMarkId: legacy.id});
  h.ref.current.setTextFontSizeLive(36);
  h.component.render();
  h.ref.current.endTextFontSizeAdjustment();
  h.component.render();
  const repaired = h.changes.at(-1)[0];
  const lines = layout.layoutTextLines(repaired.text, repaired.rect.width, value => value.length * 36 * .4);
  assert.ok(lines.length > 2, "the two paragraphs also need automatic wrapping");
  assert.equal(repaired.rect.height, Math.ceil(lines.length * 36 * 1.25));
  assert.ok(repaired.rect.height > legacy.rect.height);
  const lastLine = {x: repaired.rect.x + 10, y: repaired.rect.y + (lines.length - 1) * 36 * 1.25 + 10};
  assert.equal(model.markIndexAt([repaired], lastLine), 0);
  h.ref.current.undo();
  h.component.render();
  assert.deepEqual(h.changes.at(-1), [legacy]);
});

test("repairing bounds at the same font size is still one undoable change", () => {
  const legacy = {...text, rect: {...text.rect, height: 10}};
  const h = annotation(documentWith([legacy]), {selectedMarkId: legacy.id});
  h.ref.current.setTextFontSizeLive(18);
  h.ref.current.endTextFontSizeAdjustment();
  h.component.render();
  assert.equal(h.changes.at(-1)[0].rect.height, 45);
  h.ref.current.undo();
  h.component.render();
  assert.deepEqual(h.changes.at(-1), [legacy]);
});

test("clicking an off-center resize handle does not insert an invisible undo step", () => {
  const h = annotation(documentWith([rectangle]), {selectedMarkId: rectangle.id});
  h.pointer("onPointerDown", 140, 140);
  h.pointer("onPointerMove", 170, 150);
  h.pointer("onPointerUp", 170, 150);
  const moved = h.changes.at(-1)[0];
  assert.equal(moved.rect.x, 130);
  assert.equal(moved.rect.y, 110);
  h.pointer("onPointerDown", 183, 114); // Inside the 9px handle target, off its center.
  h.pointer("onPointerUp", 183, 114);
  assert.equal(h.changes.length, 1);
  h.ref.current.undo();
  h.component.render();
  assert.deepEqual(h.changes.at(-1), [rectangle]);
});

test("a cropped mosaic is exported using the same source bounds and document as reopening", async () => {
  const mosaic = {kind: "mosaic", id: 3, points: [{x: 200, y: 180}, {x: 460, y: 180}],
    brushDiameter: 20, intensity: "standard", style: "pixel"};
  const h = annotation(documentWith([mosaic]));
  const first = await h.ref.current.exportResult({x: 250, y: 0, width: 390, height: 360});
  assert.deepEqual(first.cropPixels, {x: 250, y: 0, width: 390, height: 360});
  assert.deepEqual(first.document.marks[0].points, [{x: -50, y: 180}, {x: 210, y: 180}]);
  const firstRender = h.exports[0];
  assert.equal(firstRender.sourceWidth, 390);
  assert.equal(firstRender.sourceHeight, 360);
  assert.deepEqual(firstRender.sourceOffset, {x: 0, y: 0});
  assert.deepEqual(firstRender.source.drawCalls[0].slice(1), [250, 0, 390, 360, 0, 0, 390, 360]);
  const reopened = annotation(first.document);
  const second = await reopened.ref.current.exportResult();
  assert.equal(second.cropPixels, null);
  assert.deepEqual(second.document, first.document);
  const reopenedRender = reopened.exports[0];
  for (const key of ["sourceWidth", "sourceHeight", "sourceOffset", "regionSize", "scaleX", "scaleY", "marks"]) {
    assert.deepEqual(reopenedRender[key], firstRender[key], key);
  }
});
