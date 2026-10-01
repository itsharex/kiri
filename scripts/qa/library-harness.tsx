// Isolated real-component layout check. No native commands or user assets.
import React from "react";
import { createRoot } from "react-dom/client";
import { LibraryWindow } from "../../src/windows/LibraryWindow";
import { setLanguage, type KiriLanguage } from "../../src/i18n";
import "../../src/styles/design-system.css";

const actions: string[] = [];
Object.assign(window, {
  __qaActions: actions,
  __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
  __TAURI_INTERNALS__: {
    metadata: { currentWindow: { label: "library" }, currentWebview: { label: "library" } },
    transformCallback: () => 1,
    unregisterCallback: () => {},
    convertFileSrc: () => "/src-tauri/tests/fixtures/qr/url.png",
    invoke: async (command: string) => {
      if (command.startsWith("plugin:event|")) return 1;
      switch (command) {
        case "get_library_status": return { availability: "ready", isDefault: true, locationLabel: "Test Library" };
        case "get_shortcut_status": return { state: "enabled", label: "Shift+Ctrl+A" };
        case "list_assets": case "list_pending_recordings": case "list_qr_favorites": case "list_text_history": return [];
        case "import_media": actions.push(command); return { ids: [], failed: 0 };
        case "paste_clipboard_image": actions.push(command); return { id: "fixture" };
        default: throw new Error(`Unexpected harness command: ${command}`);
      }
    },
  },
});
const language = new URLSearchParams(location.search).get("language") ?? "en";
setLanguage((["en", "zh-Hans", "ja"].includes(language) ? language : "en") as KiriLanguage);
createRoot(document.getElementById("root")!).render(<LibraryWindow />);
