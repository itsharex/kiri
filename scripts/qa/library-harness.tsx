// Isolated real-component layout check. No native commands or user assets.
import React from "react";
import { createRoot } from "react-dom/client";
import { LibraryWindow } from "../../src/windows/LibraryWindow";
import { setLanguage, type KiriLanguage } from "../../src/i18n";
import "../../src/styles/design-system.css";

const actions: string[] = [];
const callbacks = new Map<number, (event: unknown) => void>();
const eventHandlers = new Map<string, number>();
let nextCallback = 0;
const params = new URLSearchParams(location.search);
const savingJobs = params.has("saving") ? [
  { id: "saving-video", kind: "video", createdAt: new Date().toISOString(), duration: 125, pixelWidth: 1920, pixelHeight: 1080 },
  { id: "saving-gif", kind: "gif", createdAt: new Date().toISOString(), duration: 8, pixelWidth: 720, pixelHeight: 405 },
] : [];
const assets: unknown[] = [];
const emit = (event: string, payload: unknown) => {
  const id = eventHandlers.get(event);
  if (id != null) callbacks.get(id)?.({ event, id, payload });
};
Object.assign(window, {
  __qaActions: actions,
  __qaCompleteSave: () => {
    const job = savingJobs.shift();
    if (!job) return;
    assets.push({ id: job.id, kind: job.kind, createdAt: Date.now(),
      filename: "qa.mp4", title: "Public QA recording", ocrText: null,
      ocrOriginalText: null, qrText: null, tags: [], pixelWidth: job.pixelWidth,
      pixelHeight: job.pixelHeight, duration: job.duration, sourceApplication: null,
      isFavorite: false, trashedAt: null, gifEligible: false });
    emit("recording-save-jobs", [...savingJobs]);
    emit("library-changed", null);
  },
  __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
  __TAURI_INTERNALS__: {
    metadata: { currentWindow: { label: "library" }, currentWebview: { label: "library" } },
    transformCallback: (callback: (event: unknown) => void) => {
      const id = ++nextCallback; callbacks.set(id, callback); return id;
    },
    unregisterCallback: (id: number) => { callbacks.delete(id); },
    convertFileSrc: () => "/src-tauri/tests/fixtures/qr/url.png",
    invoke: async (command: string, args: Record<string, unknown> = {}) => {
      if (command === "plugin:event|listen") {
        eventHandlers.set(args.event as string, args.handler as number); return args.handler;
      }
      if (command.startsWith("plugin:event|")) return 1;
      switch (command) {
        case "get_library_status": return { availability: "ready", isDefault: true, locationLabel: "Test Library" };
        case "get_shortcut_status": return { state: "enabled", label: "Shift+Ctrl+A" };
        case "get_recording_save_jobs": return [...savingJobs];
        case "list_assets": return [...assets];
        case "get_asset_availability": return { status: "ready" };
        case "copy_asset": actions.push(command); return null;
        case "list_pending_recordings": case "list_qr_favorites": case "list_text_history": return [];
        case "import_media": actions.push(command); return { ids: [], failed: 0 };
        case "paste_clipboard_image": actions.push(command); return { id: "fixture" };
        default: throw new Error(`Unexpected harness command: ${command}`);
      }
    },
  },
});
const language = params.get("language") ?? "en";
setLanguage((["en", "zh-Hans", "ja"].includes(language) ? language : "en") as KiriLanguage);
createRoot(document.getElementById("root")!).render(<LibraryWindow />);
