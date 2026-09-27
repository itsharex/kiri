import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import Module from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import ts from "typescript";

const filename = fileURLToPath(new URL("../src/components/KiriIcons.tsx", import.meta.url));
const source = readFileSync(filename, "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
  fileName: filename,
});
const compiled = new Module(filename);
compiled.filename = filename;
compiled.paths = Module._nodeModulePaths(path.dirname(filename));
compiled._compile(outputText, filename);
const { KiriIcon } = compiled.exports;

test("backend error notice symbols and unknown symbols render safely", () => {
  for (const name of [
    "exclamationmark.triangle",
    "exclamationmark.triangle.fill",
    "future.error.symbol",
  ]) {
    const html = renderToStaticMarkup(React.createElement(KiriIcon, { name }));
    assert.match(html, /^<svg\b/);
  }
});
