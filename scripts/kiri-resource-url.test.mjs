import assert from "node:assert/strict";
import test, { afterEach, beforeEach } from "node:test";

import { kiriResourceUrl, configureVideoPlaybackOrigin, videoResourceUrl, videoResourceCrossOrigin } from "../src/lib/kiri-resource-url.js";

let calls;

beforeEach(() => {
  configureVideoPlaybackOrigin(null);
  calls = [];
  globalThis.window = {
    __TAURI_INTERNALS__: {
      convertFileSrc(route, protocol) {
        calls.push({ route, protocol });
        return `http://${protocol}.localhost/${encodeURIComponent(route)}`;
      },
    },
  };
});

test("Linux video bridge is limited to the IPC-provided numeric loopback capability", () => {
  const id = "00000000-0000-4000-8000-000000000000";
  const origin = "http://127.0.0.1:45321/0123456789abcdef0123456789abcdef/media";
  assert.equal(videoResourceUrl(id), `http://kiri.localhost/${encodeURIComponent(`media/${id}`)}`);
  configureVideoPlaybackOrigin(origin);
  assert.equal(videoResourceUrl(id), `${origin}/${id}`);
  assert.equal(videoResourceCrossOrigin(videoResourceUrl(id)), undefined);
  for (const source of [`kiri://media/${id}`, `http://kiri.localhost/media/${id}`, `http://127.0.0.1:45321/public.mp4`, `https://example.org/video.mp4`]) {
    assert.equal(videoResourceCrossOrigin(source), "anonymous");
  }
  assert.equal(kiriResourceUrl("media", [id]), `http://kiri.localhost/${encodeURIComponent(`media/${id}`)}`);
  for (const invalid of [origin.replace("127.0.0.1", "localhost"), origin.replace("127.0.0.1", "evil.example"), origin.replace("45321", "65536"), `${origin}?file=/etc/passwd`, origin.replace("0123456789abcdef0123456789abcdef", "guessable")]) {
    assert.throws(() => configureVideoPlaybackOrigin(invalid), TypeError);
  }
  assert.throws(() => videoResourceUrl("../private"), TypeError);
  configureVideoPlaybackOrigin(null);
  assert.equal(videoResourceUrl(id), `http://kiri.localhost/${encodeURIComponent(`media/${id}`)}`);
});

afterEach(() => {
  delete globalThis.window;
});

test("converts each complete non-empty private resource route", () => {
  const cases = [
    ["capture", ["frozen", "abc.png"]],
    ["thumbnail", ["00000000-0000-4000-8000-000000000000"]],
    ["media", ["00000000-0000-4000-8000-000000000000"]],
    ["annotation-source", ["00000000-0000-4000-8000-000000000000"]],
    ["asset", ["00000000-0000-4000-8000-000000000000"]],
  ];

  for (const [route, segments] of cases) {
    const joined = [route, ...segments].join("/");
    assert.equal(
      kiriResourceUrl(route, segments),
      `http://kiri.localhost/${encodeURIComponent(joined)}`,
    );
  }
  assert.deepEqual(
    calls,
    cases.map(([route, segments]) => ({
      route: [route, ...segments].join("/"),
      protocol: "kiri",
    })),
  );

  calls = [];
  window.__TAURI_INTERNALS__.convertFileSrc = (route, protocol) => {
    calls.push({ route, protocol });
    return `${protocol}://localhost/${encodeURIComponent(route)}`;
  };

  for (const [route, segments] of cases) {
    const joined = [route, ...segments].join("/");
    assert.equal(
      kiriResourceUrl(route, segments),
      `kiri://localhost/${encodeURIComponent(joined)}`,
    );
  }
  assert.deepEqual(
    calls,
    cases.map(([route, segments]) => ({
      route: [route, ...segments].join("/"),
      protocol: "kiri",
    })),
  );
});

test("appends encoded query parameters after route conversion", () => {
  const revision = "a".repeat(64);
  const joined = "annotation-source/00000000-0000-4000-8000-000000000000";
  assert.equal(
    kiriResourceUrl(
      "annotation-source",
      ["00000000-0000-4000-8000-000000000000"],
      { revision, page: 2 },
    ),
    `http://kiri.localhost/${encodeURIComponent(joined)}?revision=${revision}&page=2`,
  );
  assert.equal(calls.length, 1);
});

test("rejects unknown routes and path or query injection", () => {
  for (const operation of [
    () => kiriResourceUrl("unknown", ["id"]),
    () => kiriResourceUrl("media", [""]),
    () => kiriResourceUrl("media", ["."]),
    () => kiriResourceUrl("media", [".."]),
    () => kiriResourceUrl("media", ["nested/id"]),
    () => kiriResourceUrl("media", ["id\\other"]),
    () => kiriResourceUrl("media", ["id?other"]),
    () => kiriResourceUrl("media", ["id#other"]),
    () => kiriResourceUrl("media", ["id"], { "bad/key": "value" }),
  ]) {
    assert.throws(operation, TypeError);
  }
  assert.equal(calls.length, 0);
});
