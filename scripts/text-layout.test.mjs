import assert from "node:assert/strict";
import test from "node:test";

import {
  fitTextEditorFrame,
  layoutTextLines,
  textLineRuns,
  textEditorInsets,
} from "../src/annotation/text-layout.js";

const measureText = (text) => text.length * 10;

test("text layout preserves explicit and empty lines", () => {
  assert.deepEqual(layoutTextLines("A\nB", 100, measureText), ["A", "B"]);
  assert.deepEqual(layoutTextLines("A\n\nB", 100, measureText), ["A", "", "B"]);
  assert.deepEqual(layoutTextLines("  Kiri\nnext line  ", 200, measureText), [
    "  Kiri",
    "next line  ",
  ]);
});

test("text layout counts Latin and CJK soft wrapping", () => {
  assert.deepEqual(layoutTextLines("one two", 34, measureText), ["one", "two"]);
  assert.deepEqual(layoutTextLines("abcdef", 20, measureText), ["ab", "cd", "ef"]);
  assert.deepEqual(layoutTextLines("中文测试", 20, measureText), ["中文", "测试"]);
});

test("text editor frame covers wrapped lines and remains inside narrow bounds", () => {
  const multiline = fitTextEditorFrame({
    text: "one two",
    fontSize: 20,
    x: 40,
    y: 40,
    maxWidth: 50,
    boundsWidth: 50,
    boundsHeight: 100,
    measureText,
  });
  assert.deepEqual(multiline, { x: 0, y: 38, width: 50, height: 62 });

  const narrow = fitTextEditorFrame({
    text: "A\nB",
    fontSize: 20,
    x: 10,
    y: 10,
    maxWidth: 20,
    boundsWidth: 20,
    boundsHeight: 20,
    measureText,
  });
  assert.deepEqual(narrow, { x: 0, y: 0, width: 20, height: 20 });
});

test("inline text padding stays usable when a low-resolution video is enlarged",()=>{
 const options={text:"Kiri",fontSize:18,x:0,y:0,maxWidth:300,boundsWidth:640,boundsHeight:360,measureText:value=>value.length*9};
 const normal=fitTextEditorFrame(options);
 const scaled=fitTextEditorFrame({...options,fontSize:9,maxWidth:150,boundsWidth:320,boundsHeight:180,measureText:value=>value.length*4.5,uiScale:.5});
 assert.ok(Math.abs(scaled.width*2-normal.width)<=1);
 assert.ok(Math.abs(scaled.height*2-normal.height)<=1);
});

test("tab-separated columns share explicit eight-space stops in editor and export", () => {
  const first = textLineRuns("A\tB\tC", measureText);
  const second = textLineRuns("AA\tBB\tCC", measureText);
  assert.deepEqual(first.runs.map(run => run.x), [0, 80, 160]);
  assert.deepEqual(second.runs.map(run => run.x), [0, 80, 160]);
  assert.deepEqual(layoutTextLines("A\tB\tC\nAA\tBB\tCC", 200, measureText), ["A\tB\tC", "AA\tBB\tCC"]);
  const frame = fitTextEditorFrame({ text: "AA\tBB\tCC", fontSize: 18,
    x: 0, y: 0, maxWidth: 400, boundsWidth: 400, boundsHeight: 100, measureText });
  assert.ok(frame.width - 2 * textEditorInsets().x >= second.width);
});

test("plain single-line text has enough content width after textarea border and padding", () => {
  const text = "padding padding padding padding";
  const frame = fitTextEditorFrame({text, fontSize: 18, x: 0, y: 0, maxWidth: 500,
    boundsWidth: 500, boundsHeight: 100, measureText});
  const contentWidth = frame.width - 2 * textEditorInsets().x;
  assert.ok(contentWidth > measureText(text), "leave rounding room for native glyph layout");
  assert.equal(layoutTextLines(text, contentWidth, measureText).length, 1);
  assert.equal(frame.height - 2 * textEditorInsets().y, 23);
});
