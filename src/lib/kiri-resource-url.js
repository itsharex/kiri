import { convertFileSrc } from "@tauri-apps/api/core";

const ROUTES = new Set([
  "capture",
  "thumbnail",
  "annotation-source",
  "asset",
  "media",
]);

let videoPlaybackOrigin = null;

export function configureVideoPlaybackOrigin(origin) {
  if (origin == null) {
    videoPlaybackOrigin = null;
    return;
  }
  if (typeof origin !== "string" || !/^http:\/\/127\.0\.0\.1:[1-9]\d{0,4}\/[a-f0-9]{32}\/media$/.test(origin)) {
    throw new TypeError("Invalid local video playback origin.");
  }
  const url = new URL(origin);
  if (!url.port || Number(url.port) > 65535) throw new TypeError("Invalid local video playback port.");
  videoPlaybackOrigin = origin;
}

export function videoResourceUrl(id) {
  checkedSegment(id);
  return videoPlaybackOrigin ? `${videoPlaybackOrigin}/${encodeURIComponent(id)}` : kiriResourceUrl("media", [id]);
}

export function videoResourceCrossOrigin(src) {
  // Linux's read-only viewer never reads video pixels into Canvas. Avoid a
  // CORS fetch from a WebKit custom-origin document, which can serialize its
  // Origin as opaque. Other native platforms retain Canvas-safe CORS loading.
  return /^http:\/\/127\.0\.0\.1:[1-9]\d{0,4}\/[a-f0-9]{32}\/media\/[a-f0-9-]{36}$/.test(src)
    ? undefined : "anonymous";
}

function checkedSegment(segment) {
  if (
    typeof segment !== "string" ||
    segment.length === 0 ||
    segment === "." ||
    segment === ".." ||
    /[/\\?#]/.test(segment)
  ) {
    throw new TypeError("Invalid Kiri resource URL segment.");
  }
  return segment;
}

/**
 * Builds a platform-correct URL for the private `kiri` protocol.
 * Tauri percent-encodes the complete route and uses an HTTP origin on Windows,
 * so query parameters are appended only after the platform conversion.
 */
export function kiriResourceUrl(route, segments = [], query) {
  if (!ROUTES.has(route)) {
    throw new TypeError("Unknown Kiri resource URL route.");
  }
  const joinedRoute = [route, ...segments.map(checkedSegment)].join("/");
  const encodedQuery = new URLSearchParams();
  for (const [key, value] of Object.entries(query ?? {})) {
    checkedSegment(key);
    encodedQuery.append(key, String(value));
  }
  const converted = convertFileSrc(joinedRoute, "kiri");
  const suffix = encodedQuery.toString();
  return suffix ? `${converted}?${suffix}` : converted;
}
